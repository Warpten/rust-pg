use std::borrow::Cow;
use std::slice::Chunks;
use bytes::Buf;
use crate::dbc::r#impl::chunks::{Chunked, StringBlock};

pub struct WDB2<'a> {
    records: Chunked<'a>,
    sb: StringBlock<'a>,
    indices: Option<Chunked<'a>>,

    table_hash: u32,
    build: u32,
}
impl<'a> WDB2<'a> {
    pub fn read(mut cursor: &'a[u8]) -> (Self, &'a[u8]) {
        let record_count = cursor.get_u32_le() as usize;
        let _field_count = cursor.get_u32_le();
        let record_size = cursor.get_u32_le() as usize;
        let string_block_size = cursor.get_u32_le() as usize;
        let table_hash = cursor.get_u32_le();
        let build = cursor.get_u32_le();
        let _tz = cursor.get_u32_le(); // timestamp_last_write
        let min_id = cursor.get_u32_le() as usize;
        let max_id = cursor.get_u32_le() as usize;
        let _locale = cursor.get_u32_le();
        let _cp_sz = cursor.get_u32_le();

        let (indices, mut cursor) = Chunked::read_optional(max_id != 0, 4, max_id - min_id + 1, cursor);

        // Skip the block containing string lengths, it's useless
        if max_id != 0 {
            cursor.advance(2 * (max_id - min_id + 1));
        }

        let (records, cursor) = Chunked::read(record_count, record_size, cursor);
        let (sb, cursor) = StringBlock::read(string_block_size, cursor);

        (Self {
            records,
            sb,
            indices,

            table_hash,
            build,
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