use std::borrow::Cow;
use std::io::{BufRead, Read};
use std::string;
use bytes::Buf;
use crate::dbc::r#impl::RawDatabaseClientFile;
use crate::dbc::r#impl::wdb2::WDB2;
use crate::dbc::r#impl::wdb3::WDB3;
use crate::dbc::r#impl::wdb5::WDB5;
use crate::dbc::r#impl::wdb6::WDB6;
use crate::dbc::r#impl::wdbc::WDBC;
use crate::dbc::r#impl::wdc1::WDC1;

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
        match magic {
            Version::WDBC => {
                let (wdbc, cursor) = WDBC::read(cursor);
                assert!(!cursor.has_remaining(), "Error parsing WDBC");
            },
            Version::WDB2 => {
                let (wdb2, cursor) = WDB2::read(cursor);
                assert!(!cursor.has_remaining(), "Error parsing WDB2");
            },
            Version::WDB3 => {
                let (wdb3, cursor) = WDB3::read(cursor);
                assert!(!cursor.has_remaining(), "Error parsing WDB3");
            },
            Version::WDB4 => {
                panic!("Unsupported WDB4");
            },
            Version::WDB5 => {
                let (wdb5, cursor) = WDB5::read(cursor);
                assert!(!cursor.has_remaining(), "Error parsing WDB5");
            },
            Version::WDB6 => {
                let (wdb6, cursor) = WDB6::read(cursor);
                assert!(!cursor.has_remaining(), "Error parsing WDB6");
            },
            Version::WDC1 => {
                let (wdc1, cursor) = WDC1::read(cursor);
                assert!(!cursor.has_remaining(), "Error parsing WDC1");
            },
            _ => panic!()
        };
    }
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