use std::borrow::Cow;
use std::io::{BufRead, Read};
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
        let records : Vec<_> = match magic {
            Version::WDBC => {
                if let Some(wdbc) = wdbc::WDBC::new(cursor) {
                    wdbc.records().collect()
                } else {
                    todo!()
                }
            },
            _ => panic!()
        };

    }
}

mod wdbc {
    use std::io::BufRead;
    use std::ops::Range;
    use bytes::Buf;

    pub struct WDBC {
        record_count: usize,
        record_size: usize,
        string_block_size: usize,
        data: Vec<u8>,
    }
    impl WDBC {
        pub fn new(mut cursor : &[u8]) -> Option<Self> {
            let record_count = cursor.get_u32_le() as usize;
            let _field_count = cursor.get_u32_le();
            let record_size = cursor.get_u32_le() as usize;
            let string_block_size = cursor.get_u32_le() as usize;

            let data = Vec::from(&cursor[0..record_count * record_size + string_block_size]);
            cursor.advance(record_count * record_size + string_block_size);

            Some(Self {
                record_size,
                record_count,
                string_block_size,
                data
            })
        }

        pub fn records(&self) -> impl Iterator<Item = &[u8]> {
            self.data[..]
                .chunks(self.record_size)
                .take(self.record_count)
        }

        pub fn record(&self, index: usize) -> Option<&[u8]> {
            if index < self.record_count {
                Some(&self.data[Range {
                    start: self.record_size * index,
                    end: self.record_size * (index + 1)
                }])
            } else {
                None
            }
        }

        fn string(&self, index: usize) -> Option<&[u8]> {
            self.data[Range {
                start: self.record_size * self.record_count,
                end: self.record_size * self.record_count + self.string_block_size
            }].split(|b| *b == 0)
                .skip(index)
                .find(|_| true)
        }

        fn record_count(&self) -> usize { self.record_count }
    }
}
#[repr(u32)]
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

trait Record {

}