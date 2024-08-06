use std::{collections::HashMap, ops::Range};

use bytemuck::cast_slice;
use bytes::Buf;

use crate::dbcd::{dbd::{Definition, StructureDefinition}, raw::{Chunked, ChunkedBuf, ChunkedTrait, Raw, RawTrait}, structured::{Common, ExtendedFieldInfo, FieldCompressionCategory, FieldInfo, Relation}, typed::table::Table};

use super::shared::{RawValue, DBC};

pub struct WDC5<'a> {
    sections: Vec<Section<'a>>,
    fields: Chunked<'a>,
    field_info: Chunked<'a>,
    pallet_data: Raw<'a>,
    common_data: Raw<'a>,

    record_count: usize,
    layout_hash: u32,
}
impl WDC5<'_> {
    pub fn new(source: &[u8]) -> WDC5 {
        let mut cursor = source;
        
        let version = cursor.get_u32_le();
        assert_eq!(version, 5);

        cursor.advance(128); // Skip schema_string
        let record_count = cursor.get_u32_le() as usize;
        let _field_count = cursor.get_u32_le();
        let record_size = cursor.get_u32_le() as usize;
        let _string_table_size = cursor.get_u32_le() as usize;
        let _table_hash = cursor.get_u32_le();
        let layout_hash = cursor.get_u32_le();
        let _min_id = cursor.get_u32_le();
        let _max_id = cursor.get_u32_le();
        let _locale = cursor.get_u32_le();
        let flags = cursor.get_u16_le();
        let _id_index = cursor.get_u16_le();
        let total_field_count = cursor.get_u32_le() as usize;
        let _bitpacked_data_offset = cursor.get_u32_le();
        let _lookup_column_count = cursor.get_u32_le();
        let field_storage_info_size = cursor.get_u32_le() as usize;
        let common_data_size = cursor.get_u32_le() as usize;
        let pallet_data_size = cursor.get_u32_le() as usize;
        let section_count = cursor.get_u32_le();

        let section_headers: Vec<_> = (0..section_count).map(|_| {
            let tact_key_hash = cursor.get_u64();
            let file_offset = cursor.get_u32_le() as usize;
            let record_count = cursor.get_u32_le() as usize;
            let string_table_size = cursor.get_u32_le() as usize;
            let offset_records_end = cursor.get_u32_le() as usize;
            let id_list_size = cursor.get_u32_le() as usize;
            let relationship_data_size = cursor.get_u32_le() as usize;
            let offset_map_id_count = cursor.get_u32_le() as usize;
            let copy_table_count = cursor.get_u32_le() as usize;

            SectionHeader {
                tact_key_hash,
                file_offset,
                record_count,
                string_table_size,
                offset_records_end,
                id_list_size,
                relationship_data_size,
                offset_map_id_count,
                copy_table_count,
            }
        }).collect();

        let (fields, cursor) = Chunked::from_raw(cursor, 2 + 2, total_field_count);
        let (field_info, cursor) = Chunked::from_raw(cursor, 2 + 2 + 4 + 4 + 3 * 4, field_storage_info_size / (2 + 2 + 4 + 4 + 3 * 4));
        let (pallet_data, cursor) = Raw::from_raw(cursor, pallet_data_size);
        let (common_data, mut cursor) = Raw::from_raw(cursor, common_data_size);

        // Iterate sections; read encryption status for those where key != 0
        let encrypted_ids: Vec<_> = section_headers.iter()
            .map(|section_header| {
                if section_header.tact_key_hash != 0 {
                    let encrypted_id_count = cursor.get_u32_le() as usize;
                    let (encrypted_ids, after) = ChunkedBuf::from_raw(cursor, 4, encrypted_id_count);

                    cursor = after;

                    Some(encrypted_ids)
                } else {
                    None
                }
            }).collect();

        // Now sections
        let sections: Vec<_> = section_headers
            .into_iter()
            .zip(encrypted_ids.into_iter())
            .map(|(section_header, encrypted)| {
                let (records, cursor) = if (flags & 0x01) == 0 {
                    Raw::from_raw(cursor, record_size * section_header.record_count)
                } else {
                    Raw::from_raw(cursor, section_header.offset_records_end - section_header.file_offset)
                };

                let (string_block, cursor) = Raw::optionally_from((flags & 0x01) == 0, cursor, section_header.string_table_size);
                let (id_list, cursor) = Raw::from_raw(cursor, section_header.id_list_size / 4);
                let (copy_table, cursor) = Chunked::from_raw(cursor, 8, section_header.copy_table_count);
                let (offset_map, mut cursor) = Chunked::from_raw(cursor, 4 + 2, section_header.offset_map_id_count);
                let (relation, after) = Raw::from_raw(cursor, section_header.relationship_data_size);

                cursor = after;

                Section {
                    tact_key_hash: section_header.tact_key_hash,
                    encrypted,
                    records,
                    string_block,
                    id_list,
                    copy_table,
                    offset_map,
                    relation,
                }
            }).collect();

        WDC5 {
            sections,
            fields,
            field_info,
            pallet_data,
            common_data,

            layout_hash,
            record_count,
        }
    }
}

impl DBC for WDC5<'_> {
    fn parse_table(self, definition: &Definition) -> Option<Table> {
        if let Some(spec) = definition.select(Some(self.layout_hash), None) {
            let (records, ids) = Parser::new(self).parse(definition, spec);

            Some(Table::new(spec, records, ids))
        } else {
            None
        }
    }

    fn parse(self, definition: &Definition) -> Vec<Vec<RawValue>> {
        if let Some(spec) = definition.select(Some(self.layout_hash), None) {
            let (records, _) = Parser::new(self).parse(definition, spec);

            records
        } else {
            vec![]
        }
    }
}

struct Parser<'a> {
    field_info: Vec<ExtendedFieldInfo>,

    sections: Vec<SectionParser<'a>>,
    pallet_data: Vec<u8>,
    common_data: Common,

    record_count: usize,
}

struct SectionParser<'a> {
    ids: Vec<u32>,
    copy_table: HashMap<u32, u32>,
    relation: Relation,
    records: Vec<(&'a [u8], usize)>,
}

impl Parser<'_> {
    pub fn new(data: WDC5) -> Parser {
        // Collect field info
        let field_info : Vec<_> = (0..data.fields.len()).map(|i| {
            FieldInfo::new(data.fields.at(i), data.field_info.at(i))
        }).collect();

        // Collect additional data ranges (for common and pallet)
        let additional_data_ranges = {
            let mut result = vec![];

            let mut category_offsets = [0; FieldCompressionCategory::MAX as usize];
            let mut category_indices = [0; FieldCompressionCategory::MAX as usize];
            field_info.iter().for_each(|field_info| {
                // We do a bit of extra work but can't skip those that have no additional
                // size because both iterations would not line up when zipped below. We
                // will get an empty range anyways.

                let offset = &mut category_offsets[field_info.storage_type.category() as usize];
                let range = Range {
                    start: *offset,
                    end: *offset + field_info.additional_data_size
                };

                let index = &mut category_indices[field_info.storage_type.category() as usize];
                
                *offset = range.end; // `range` is moved-from next line
                result.push((range, *index));
                
                *index += 1;
            });

            result
        };

        // Zip fields and their additional data ranges
        let field_info: Vec<_> = field_info.into_iter()
            .zip(additional_data_ranges.into_iter())
            .map(|(f, (r, i))| {
                ExtendedFieldInfo {
                    field: f,
                    additional_data_range: r,
                    category_index: i
                }
            })
            .collect();

        // Parse the common block
        let common = Common::new(data.common_data.data(), &field_info);

        let sections: Vec<_> = data.sections.iter().map(|section| {
            let ids: &[u32] = cast_slice(&section.id_list.data());
            let copy_table: HashMap<_, _> = section.copy_table.materialize(|mut segment| {
                let from = segment.get_u32_le();
                let to = segment.get_u32_le();

                (from, to)
            }).collect();

            let relation = Relation::new(section.relation.data());

            SectionParser {
                ids: ids.to_vec(),
                copy_table,
                relation,
                records: section.offset_map.materialize(|mut chunk| {
                    let offset = chunk.get_u32_le() as usize;
                    let size = chunk.get_u16_le() as usize;

                    let range = Range {
                        start: offset,
                        end: size + offset
                    };

                    (&section.records[range], offset)
                }).collect()
            }
        }).collect();

        Parser {
            field_info,

            sections,
            pallet_data: data.pallet_data.to_vec(),
            common_data: common,

            record_count: data.record_count,
        }
    }
    
    pub fn parse(&self, definition: &Definition, spec: &StructureDefinition) -> (Vec<Vec<RawValue>>, Vec<u32>) {
        let mut rows: Vec<()> = Vec::with_capacity(self.record_count);

        self.sections.into_iter().map(|section| {
            let section_rows: Vec<_> = section.records.into_iter().map(|(rec, rec_ofs)| {

            }).collect();

            todo!()
        });

        todo!()
    }
}

struct Section<'a> {
    tact_key_hash: u64,
    encrypted: Option<ChunkedBuf>,
    records: Raw<'a>,
    string_block: Raw<'a>,
    id_list: Raw<'a>,
    copy_table: Chunked<'a>,
    offset_map: Chunked<'a>,
    relation: Raw<'a>
}

struct SectionHeader {
    tact_key_hash: u64,
    file_offset: usize,
    record_count: usize,
    string_table_size: usize,
    offset_records_end: usize,
    id_list_size: usize,
    relationship_data_size: usize,
    offset_map_id_count: usize,
    copy_table_count: usize,
}
