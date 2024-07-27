use std::borrow::Cow;
use std::slice::Chunks;
use bytes::Buf;
use crate::dbc::r#impl::chunks::{Chunked, StringBlock};

pub struct WDBC<'a> {
    records: Chunked<'a>,
    sb: StringBlock<'a>,
}
impl<'a> WDBC<'a> {
    pub fn read(mut cursor: &'a[u8]) -> (Self, &'a[u8]) {
        let record_count = cursor.get_u32_le() as usize;
        let _field_count = cursor.get_u32_le();
        let record_size = cursor.get_u32_le() as usize;
        let string_block_size = cursor.get_u32_le() as usize;

        let (records, cursor) = Chunked::read(record_count, record_size, cursor);
        let (sb, cursor) = StringBlock::read(string_block_size, cursor);

        (Self {
            records,
            sb
        }, cursor)
    }

    pub fn records(&self) -> Chunks<'a, u8> {
        self.records.chunks()
    }

    pub fn string(&self, idx: usize) -> Option<String> {
        unsafe {
            self.sb.get(idx).map(|bytes| std::str::from_utf8_unchecked(bytes).to_owned())
        }
    }
}