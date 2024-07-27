use std::borrow::Cow;
use std::slice::Chunks;
use bytes::Buf;
use crate::dbc::r#impl::chunks::{Chunked, StringBlock};

pub struct WDB3<'a> {
    offset_map: Chunked<'a>,
    relationship_ids: Chunked<'a>,
    records: Chunked<'a>,
    sb: StringBlock<'a>,
    ids: Chunked<'a>,
    copy_table: Option<Chunked<'a>>,

    table_hash: u32,
    build: u32
}
impl<'a> WDB3<'a> {
    pub fn read(mut cursor: &'a[u8]) -> (Self, &'a[u8]) {
        let record_count = cursor.get_u32_le() as usize;
        let _field_count = cursor.get_u32_le();
        let record_size = cursor.get_u32_le() as usize;
        let string_block_size = cursor.get_u32_le() as usize;
        let table_hash = cursor.get_u32_le();
        let build = cursor.get_u32_le();
        let _tz_last_written = cursor.get_u32_le();
        let min_id = cursor.get_u32_le() as usize;
        let max_id = cursor.get_u32_le() as usize;
        let _locale = cursor.get_u32_le();
        let cp_sz = cursor.get_u32_le() as usize / 8;

        let (offset_map, cursor) = Chunked::read(4 + 2, max_id - min_id + 1, cursor);
        let (relationship_ids, cursor) = Chunked::read(4, max_id - min_id + 1, cursor);
        let (records, cursor) = Chunked::read(record_count, record_size, cursor);
        let (sb, cursor) = StringBlock::read(string_block_size, cursor);
        let (ids, cursor) = Chunked::read(4, record_count, cursor);
        let (copy_table, cursor) = Chunked::read_optional(cp_sz > 0, 4 + 4, cp_sz, cursor);

        (Self {
            offset_map,
            relationship_ids,
            records,
            sb,
            ids,
            copy_table,

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