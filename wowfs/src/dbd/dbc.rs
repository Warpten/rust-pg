use std::borrow::Cow;
use std::io::{BufRead, Read};
use std::string;
use bytes::Buf;

pub struct DBC {

}
impl DBC {
    pub fn read<S>(source : &[u8], name: S)
        where S : AsRef<str>
    {
        let mut cursor = source;

        let magic = unsafe {
            std::mem::transmute::<_, Version>(cursor.get_u32_le())
        };
        let implementation = match magic {
            Version::WDBC => {
                let record_count = cursor.get_u32_le() as usize;
                let _field_count = cursor.get_u32_le();
                let record_size = cursor.get_u32_le() as usize;
                let string_block_size = cursor.get_u32_le() as usize;

                let (records, mut cursor) = blocks::Records::read(record_count, record_size, cursor);
                let (sb, mut cursor) = blocks::StringBlock::read(string_block_size, cursor);
            },
            Version::WDB2 => {
                let record_count = cursor.get_u32_le() as usize;
                let _field_count = cursor.get_u32_le();
                let record_size = cursor.get_u32_le() as usize;
                let string_block_size = cursor.get_u32_le() as usize;
                let _table_hash = cursor.get_u32_le();
                let _build = cursor.get_u32_le();
                let _tz = cursor.get_u32_le(); // timestamp_last_write
                let min_id = cursor.get_u32_le() as usize;
                let max_id = cursor.get_u32_le() as usize;
                let _locale = cursor.get_u32_le();
                let _cp_sz = cursor.get_u32_le();

                let (indices, string_lengths, cursor) = if max_id != 0 {
                    let (indices, mut cursor) = blocks::Integers::<4>::read(max_id - min_id + 1, cursor);
                    let (string_lengths, mut cursor) = blocks::Integers::<2>::read(max_id - min_id + 1, cursor);
                    
                    (Some(indices), Some(string_lengths), cursor)
                } else {
                    (None, None, cursor)
                };

                let (records, mut cursor) = blocks::Records::read(record_count, record_size, cursor);
                let (sb, mut cursor) = blocks::StringBlock::read(string_block_size, cursor);

            },
            Version::WDB3 => {
                let record_count = cursor.get_u32_le() as usize;
                let _field_count = cursor.get_u32_le();
                let record_size = cursor.get_u32_le() as usize;
                let string_block_size = cursor.get_u32_le() as usize;
                let _table_hash = cursor.get_u32_le();
                let _build = cursor.get_u32_le();
                let _tz = cursor.get_u32_le(); // timestamp_last_write
                let min_id = cursor.get_u32_le() as usize;
                let max_id = cursor.get_u32_le() as usize;
                let _locale = cursor.get_u32_le();
                let cp_sz = cursor.get_u32_le() as usize;

                let (offset_map, mut cursor) = blocks::OffsetMap::read(max_id - min_id + 1, cursor);
                let (relationship_ids, mut cursor) = blocks::Integers::<4>::read(max_id - min_id + 1, cursor);
                let (records, mut cursor) = blocks::Records::read(record_count, record_size, cursor);
                let (sb, mut cursor) = blocks::StringBlock::read(string_block_size, cursor);
                let (id_table, mut cursor) = blocks::Integers::<4>::read(record_count, cursor);
                let (copy_table, mut cursor) = blocks::CopyTable::read(cp_sz, cursor);

            }
            _ => panic!()
        };

        assert!(cursor.remaining() == 0, "Error parsing {:?}", magic);
    }
}

mod blocks {
    use std::{ops::{Index, Range}, slice::Chunks};

    pub struct Records<'a> {
        record_size: usize,
        data: &'a[u8],
    }
    impl<'a> Records<'a> {
        pub fn read(record_size: usize, record_count: usize, cursor: &'a [u8]) -> (Records, &'a[u8]) {
            let (data, remainder) = cursor.split_at(record_size * record_count);

            (Self { record_size, data }, remainder)
        }

        pub fn records(&self) -> Chunks<'a, u8> {
            self.data.chunks(self.record_size)
        }
    }

    pub struct StringBlock<'a> {
        data: &'a[u8],
        offsets: Vec<usize>
    }
    impl<'a> StringBlock<'a> {
        pub fn read(sz: usize, data: &'a[u8]) -> (Self, &'a[u8]) {
            let (block, remainder) = data.split_at(sz);

            let offsets : Vec<_> = block.into_iter()
                .enumerate()
                .filter(|&(_, b)| *b == 0)
                .map(|(i, _)| i)
                .collect();

            (Self { data, offsets }, remainder)
        }
    }
    impl Index<usize> for StringBlock<'_> {
        type Output = [u8];
    
        fn index(&self, index: usize) -> &Self::Output {
            self.offsets.get(index).map(|i| {
                let range = Range {
                    start: index,
                    end: if index + 1 >= self.offsets.len() {
                        self.offsets.len()
                    } else {
                        self.offsets[index + 1] - 1
                    }
                };

                &self.data[range]
            }).unwrap_or(&[])
        }
    }

    // General purpose block of integer types of [`N`] bytes.
    pub struct Integers<'a, const N : usize> {
        data: &'a [u8],
    }
    impl<'a, const N : usize> Integers<'a, N> {
        pub fn read(count: usize, data: &'a[u8]) -> (Self, &'a[u8]) {
            let (data, rem) = data.split_at(count * N);
            (Self { data }, rem)
        }
    }

    pub struct OffsetMap<'a> {
        data: &'a[u8],
        count: usize
    }
    impl<'a> OffsetMap<'a> {
        pub fn read(count: usize, data: &'a[u8]) -> (Self, &'a[u8]) {
            let (data, rem) = data.split_at(count * (4 + 2));
            (Self { data, count }, rem)
        }
    }

    pub type CopyTable = Integers::<8>;
}

mod wdb {
    use std::slice::Chunks;
    use bytes::Buf;

    pub struct WDBC<'a> {
        record_count: usize,
        record_size: usize,
        data: &'a[u8],
        sb: Vec<&'a[u8]>,
    }
    impl<'a> WDBC<'a> {
        pub fn new(mut cursor : &'a [u8]) -> Option<Self> {
            let record_count = cursor.get_u32_le() as usize;
            let _field_count = cursor.get_u32_le();
            let record_size = cursor.get_u32_le() as usize;
            let string_block_size = cursor.get_u32_le() as usize;

            let data = &cursor[0..record_count * record_size + string_block_size];
            cursor.advance(record_count * record_size);
            let sb = cursor[0..string_block_size]
                .split(|b| *b == 0_u8)
                .collect();

            Some(WDBC::<'a> {
                record_size,
                record_count,
                data,
                sb
            })
        }

        pub fn records(&self) -> Chunks<'_, u8>{
            self.data.chunks(self.record_size)
        }

        pub fn strings(&self) -> &[&[u8]] {
            &self.sb
        }
    }

    pub struct WDB2<'a> {
        record_count: usize,
        record_size: usize,
        data: &'a[u8],
        sb: Vec<&'a[u8]>,

    }
    impl<'a> WDB2<'a> {
        pub fn new(mut source : &'a [u8]) -> Option<WDB2> {

            None
        }
    }
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Version {
    WDBC = u32::from_be_bytes(*b"WDBC"),
    WDB2 = u32::from_be_bytes(*b"WDB2"),
    WDB3 = u32::from_be_bytes(*b"WDB3"),
    WDB4 = u32::from_be_bytes(*b"WDB4"),
    WDB5 = u32::from_be_bytes(*b"WDB5"),
    WDB6 = u32::from_be_bytes(*b"WDB6"),
    WDC1 = u32::from_be_bytes(*b"WDC1"),
    WDC2 = u32::from_be_bytes(*b"WDC2"),
    WDC3 = u32::from_be_bytes(*b"WDC3"),
    WDC4 = u32::from_be_bytes(*b"WDC4"),
    WDC5 = u32::from_be_bytes(*b"WDC5"),
}

trait Implementation {
}