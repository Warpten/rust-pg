use bytes::Buf;
use crate::dbc::r#impl::chunks::{Chunked, Raw, StringBlock};

pub struct WDC1<'a> {
    field_structure: Chunked<'a>,
    content: Content<'a>,
    id_list : Chunked<'a>,
    copy_table : Option<Chunked<'a>>,
    field_info: Chunked<'a>,
    pallet: Raw<'a>,
    common: Raw<'a>,
    relationship: Option<Raw<'a>>,
}
impl<'a> WDC1<'a> {
    pub fn read(mut cursor: &'a[u8]) -> (Self, &'a[u8]) {
        let record_count = cursor.get_u32_le() as usize;
        let field_count = cursor.get_u32_le() as usize;
        let record_size = cursor.get_u32_le() as usize;
        let string_block_size = cursor.get_u32_le() as usize;
        let table_hash = cursor.get_u32_le();
        let layout_hash = cursor.get_u32_le();
        let min_id = cursor.get_u32_le() as usize;
        let max_id = cursor.get_u32_le() as usize;
        let _locale = cursor.get_u32_le();
        let copy_table_count = cursor.get_u32_le() as usize / 8;
        let flags = cursor.get_u16_le();
        let id_index = cursor.get_u16_le();
        let total_field_count = cursor.get_u32_le() as usize;
        let bitpacked_data_offset = cursor.get_u32_le();
        let lookup_column_count = cursor.get_u32_le();
        let offset_map_offset = cursor.get_u32() as usize;
        let id_list_size = cursor.get_u32_le() as usize;
        let field_storage_info_count = cursor.get_u32_le() as usize / (2 + 2 + 4 + 4 + 3 * 4);
        let common_data_size = cursor.get_u32_le() as usize;
        let pallet_data_size = cursor.get_u32_le() as usize;
        let relationship_data_size = cursor.get_u32_le() as usize;

        let (field_structure, cursor) = Chunked::read(2 + 2, total_field_count, cursor);
        let (content, cursor) = if (flags & 1) != 0 {
            let (records, cursor) = Chunked::read(record_size, record_count, cursor);
            let (sb, cursor) = StringBlock::read(string_block_size, cursor);

            (Content::Normal { records, sb }, cursor)
        } else {
            let (records, cursor) = Raw::read(
                offset_map_offset - field_structure.byte_len() - 84, // 84 is the header size in bytes
                cursor
            );
            let (offset_map, cursor) = Chunked::read(4 + 2, max_id - min_id + 1, cursor);

            (Content::OffsetMap { offset_map, records }, cursor)
        };

        let (id_list, cursor) = Chunked::read(4, id_list_size / 4, cursor);
        let (copy_table, cursor) = Chunked::read_optional(copy_table_count > 0, 4 + 4, copy_table_count, cursor);
        let (field_info, cursor) = Chunked::read(2 + 2 + 4 + 4 + 3 * 4, field_storage_info_count, cursor);
        let (pallet, cursor) = Raw::read(pallet_data_size, cursor);
        let (common, cursor) = Raw::read(common_data_size, cursor);
        let (relationship, cursor) = Raw::read_optional(relationship_data_size > 0, relationship_data_size, cursor);

        (Self {
            field_structure,
            content,
            id_list,
            copy_table,
            field_info,
            pallet,
            common,
            relationship,
        }, cursor)
    }
}

enum Content<'a> {
    Normal {
        records: Chunked<'a>,
        sb: StringBlock<'a>,
    },
    OffsetMap {
        records: Raw<'a>,
        offset_map: Chunked<'a>
    }
}