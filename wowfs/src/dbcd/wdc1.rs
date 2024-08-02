use std::collections::HashMap;
use std::ops::Range;

use bytemuck::cast_slice;
use bytes::Buf;
use smallvec::{smallvec, SmallVec, ToSmallVec};

use crate::dbcd::dbd::{ColumnDefinition, ColumnReference, ColumnType, Definition};

use crate::dbcd::structured::FieldCompressionType;
use crate::dbcd::{raw::{Chunked, Raw}, structured::FieldInfo};
use crate::dbcd::wdc1::chunks::Content;

use super::dbd::StructureDefinition;
use super::shared::{self, Parser as ParserTrait, RawValue};
use super::structured::{Common, FieldCompressionCategory};

pub struct WDC1<'a> {
    fields: Chunked<'a>,
    records: Content<'a>,
    id_list: Raw<'a>,
    copy_table: Chunked<'a>,
    field_info : Chunked<'a>,
    pallet: Raw<'a>,
    common: Raw<'a>,
    relationship: Raw<'a>,

    record_count: usize,
    table_hash: u32,
    layout_hash: u32,
}
impl WDC1<'_> {
    pub fn new<'a>(source: &'a [u8]) -> WDC1<'a> {
        let mut cursor = source;

        let record_count = cursor.get_u32_le() as usize;
        let _field_count = cursor.get_u32_le() as usize;
        let record_size = cursor.get_u32_le() as usize;
        let string_table_size = cursor.get_u32_le() as usize;
        let table_hash = cursor.get_u32_le();
        let layout_hash = cursor.get_u32_le();
        let min_id = cursor.get_u32_le() as usize;
        let max_id = cursor.get_u32_le() as usize;
        let _locale = cursor.get_u32_le() as usize;
        let copy_table_size = cursor.get_u32_le() as usize;
        let flags = cursor.get_u16_le() as usize;
        let id_index = cursor.get_u16_le() as usize;
        let total_field_count = cursor.get_u32_le() as usize;
        let _bitpacked_data_offset = cursor.get_u32_le() as usize;
        let _lookup_column_count = cursor.get_u32_le() as usize;
        let offset_map_offset = cursor.get_u32_le() as usize;
        let id_list_size = cursor.get_u32_le() as usize;
        let field_storage_info_size = cursor.get_u32_le() as usize;
        let common_data_size = cursor.get_u32_le() as usize;
        let pallet_data_size = cursor.get_u32_le() as usize;
        let relationship_data_size = cursor.get_u32_le() as usize;

        let (fields, cursor) = Chunked::from(cursor, 2 + 2, total_field_count);

        let (records, cursor) = if (flags & 0x01) == 0 {
            let (records, cursor) = Chunked::from(cursor, record_size, record_count);
            let (string_block, cursor) = Raw::from(cursor, string_table_size);

            (Content::Regular { records, string_block }, cursor)
        } else {
            let distance = unsafe {
                cursor.as_ptr().offset_from(source.as_ptr()) as usize
            };

            let (records, cursor) = Raw::from(cursor, offset_map_offset - distance);
            let (offset_map, cursor) = Chunked::from(cursor, 4 + 2, max_id - min_id + 1);

            (Content::OffsetMap { records, offset_map }, cursor)
        };

        let (id_list, cursor) = Raw::from(cursor, id_list_size);
        let (copy_table, cursor) = Chunked::optionally_from(copy_table_size > 0, cursor, 4 + 4, copy_table_size / 8);
        let (field_info, cursor) = Chunked::from(cursor, 2 + 2 + 4 + 4 + 3 * 4, field_storage_info_size / (2 + 2 + 4 + 4 + 3 * 4));
        let (pallet, cursor) = Raw::from(cursor, pallet_data_size);
        let (common, cursor) = Raw::from(cursor, common_data_size);
        let (relationship, cursor) = Raw::from(cursor, relationship_data_size);

        assert!(!cursor.has_remaining());

        WDC1 {
            fields,
            records,
            id_list,
            copy_table,
            field_info,
            pallet,
            common,
            relationship,

            record_count,
            table_hash,
            layout_hash,
        }
    }

    pub fn parse(self, definition: &Definition) -> Vec<Vec<RawValue>> {
        if let Some(spec) = definition.select(Some(self.layout_hash), None) {
            let data = Parser::new(self).parse(definition, spec);

            // Transpose
            for i in 0..spec.len() {
                assert_eq!(data[0].len(), data[i].len(), "Non-rectangular record table");
            }

            let len = data[0].len();
            let mut iters: Vec<_> = data.into_iter().map(|n| n.into_iter()).collect();
            (0..len).map(|_| {
                iters.iter_mut()
                    .map(|n| n.next().unwrap())
                    .collect::<Vec<_>>()
            }).collect()
        } else {
            vec![]
        }
    }
}

pub struct Parser<'a> {
    field_info: Vec<(FieldInfo, Range<usize>)>,
    pallet: Vec<u8>,
    common: Common,
    ids: Vec<u32>,

    records: Content<'a>,
}
impl Parser<'_> {
    pub fn new(data: WDC1) -> Parser {
        // Collect field info
        let field_info : Vec<_> = (0..data.fields.len()).map(|i| {
            FieldInfo::new(data.fields.at(i), data.field_info.at(i))
        }).collect();

        // Collect additional data ranges (for common and pallet)
        let additional_data_ranges = {
            let mut result = vec![];

            let mut category_offsets = [0; FieldCompressionCategory::MAX as usize];
            field_info.iter().for_each(|field_info| {
                // We do a bit of extra work but can't skip those that have no additional
                // size because both iterations would not line up when zipped below. We
                // will get an empty range anyways.

                let value = &mut category_offsets[field_info.storage_type.category() as usize];
                let range = Range {
                    start: *value,
                    end: *value + field_info.additional_data_size
                };

                *value = range.end;
                result.push(range);
            });

            result
        };

        // Zip fields and their additional data ranges
        let field_info: Vec<_> = field_info.into_iter().zip(additional_data_ranges.into_iter()).collect();

        // Parse the common block
        let common = Common::new(data.common.data(), true);

        // Jumping through some hoops to make sure the id list is aligned properly.
        let aligned_id_list = data.id_list.data().to_vec();

        Parser {
            field_info,
            pallet: data.pallet.data().to_vec(),
            ids: cast_slice::<_, u32>(&aligned_id_list).to_vec(),
            common,

            records: data.records,
        }
    }
}

impl shared::Parser for Parser<'_> {
    /// Returns all values for all columns of all rows.
    /// The buffer returned has double indexing based on row index and column index.
    /// The first index is the column index; the second index is the row index.
    /// 
    /// TL;DR: Read with `result[column_index][record_index]`.
    fn parse(&self, definition: &Definition, structure: &StructureDefinition) -> Vec<Vec<RawValue>> {
        let records: Vec<_> = match &self.records {
            Content::Regular { records, .. } => {
                // Effectively just an array of pointers over contiguous memory
                records.iter().collect()
            },
            Content::OffsetMap { records, offset_map } => {
                offset_map.materialize(|mut chunk| {
                    let offset = chunk.get_u32_le() as usize;
                    let size = chunk.get_u16_le() as usize;

                    let range = Range {
                        start: offset,
                        end: size + offset
                    };

                    &records[range]
                }).collect()
            }
        };

        let mut values = vec![];

        let mut inline_index = 0;
        structure.iter().map(|column_reference| {
            if column_reference.id && column_reference.noninline {
                // Fast path for this kind of column - but is it correct?
                self.ids.iter()
                    .map(|&i| RawValue::U32(smallvec![i]))
                    .collect()
            } else if !column_reference.noninline {
                // ...Find the specification
                let spec = unsafe {
                    // SAFETY: this should never fail; the file is malformed otherwise.
                    definition.spec(column_reference).unwrap_unchecked()
                };

                let (field_info, additional_data_range) = &self.field_info[inline_index];
                
                // ...For each record, read the column's values
                let values : Vec<_> = records.iter().map(|record| {
                    self.parse_value(record, field_info, column_reference, spec, additional_data_range)
                }).collect();

                inline_index += 1;

                values
            } else if column_reference.relation {
                panic!("Unsupported relationship column")
            } else {
                panic!("Unknown column kind")
            }
        }).for_each(|v| values.push(v));

        values
    }
}

impl Parser<'_> {
    /// Reads a value from the provided record.
    /// 
    /// # Arguments
    /// 
    /// * `record` - The entire record data.
    /// * `field_info` - Field metadata for the column according to the WDC1 file.
    /// * `column` - A [`ColumnReference`] provided by the DBD schema.
    /// * `spec` - A [`ColumnDefinition`] provided by the DBD schema.
    pub fn parse_value(&self,
        record: &[u8],
        field_info: &FieldInfo,
        column: &ColumnReference,
        spec: &ColumnDefinition,
        additional_data_range: &Range<usize>,
    ) -> RawValue {
        assert_eq!(field_info.arity(), column.arity, "{:#?}", (column, field_info, spec));

        let byte_offset = field_info.offset_bytes();
        let byte_width = field_info.size_bytes();
        assert!(byte_width != 0, "Impossible field width {:#?}", field_info);

        let range = Range {
            start: byte_offset,
            end: byte_offset + byte_width
        };

        let raw_bytes = &record[range];
        self.transform_values_raw(raw_bytes, field_info, column, spec, additional_data_range)
    }

    fn encapsulate_raw(&self, values: &[u8], column: &ColumnReference, spec: &ColumnDefinition) -> RawValue {
        match &spec.column_type {
            // LocalizedString sometimes implies [16] but there's no way to know that
            // so just throw hands and pretend it's a string.
            ColumnType::String | ColumnType::LocalizedString => {
                match &self.records {
                    Content::Regular { string_block, .. } => {
                        let indices: &[u32] = cast_slice(values);
                        let values: Vec<_> = indices.iter().map(|&i| unsafe {
                            let mut range = Range {
                                start: i as usize,
                                end: i as usize
                            };

                            while string_block[range.end] != 0 {
                                range.end += 1;
                            }

                            String::from_utf8_unchecked(string_block[range].to_vec())
                        }).collect();

                        RawValue::String(SmallVec::from_vec(values))
                    },
                    Content::OffsetMap { .. } => {
                        // Strings are inlined...
                        let slices : Vec<_> = values.split(|&c| c == 0)
                            .map(|slice| unsafe {
                                String::from_utf8_unchecked(slice.to_vec())
                            })
                            .collect();

                        RawValue::String(SmallVec::from_vec(slices))
                    },
                }
            },
            ColumnType::Integer => {
                // This works if column.bits <= bitness of the values, but fails otherwise
                // (because it becomes a non-narrowing conversion)
                match (column.bits.unwrap_or(32), column.unsigned) {
                    (0..=8,  false) => RawValue::I8 (cast_slice(values).to_smallvec()),
                    (9..=16, false) => RawValue::I16(cast_slice(values).to_smallvec()),
                    (0..=32, false) => RawValue::I32(cast_slice(values).to_smallvec()),
                    (0..=64, false) => RawValue::I64(cast_slice(values).to_smallvec()),
                    (0..=8,  true)  => RawValue::U8 (cast_slice(values).to_smallvec()),
                    (9..=16, true)  => RawValue::U16(cast_slice(values).to_smallvec()),
                    (0..=32, true)  => RawValue::U32(cast_slice(values).to_smallvec()),
                    (0..=64, true)  => RawValue::U64(cast_slice(values).to_smallvec()),
                    (bits, _) => panic!("Unsupported {} bit width", bits)
                }
            },
            ColumnType::Float => {
                RawValue::F32(cast_slice(values).to_smallvec())
            },
        }
    }

    fn read_bitpacked(values: &[u8], field_info: &FieldInfo,) -> Vec<u8> {
        // At most, 64 bits, aka 8 bytes
        let byte_width = field_info.size_bytes();
        let shift_offset = 64 - field_info.size_bits() - (field_info.offset_bits() % 8);
        let mask_offset = 64 - field_info.size_bits();

        let mut cursor = values;

        let mut shifted_slice = Vec::with_capacity(values.len());
        for _ in 0..(values.len() / byte_width) {
            let raw_integer = cursor.get_uint_le(byte_width);
            let value = &[(raw_integer << shift_offset) >> mask_offset];
            let byte_slice :&[u8] = cast_slice(value);
            byte_slice[0..byte_width].iter()
                .for_each(|&i| shifted_slice.push(i));
        }

        shifted_slice
    }

    fn transform_values_raw(&self,
        values: &[u8],
        field_info: &FieldInfo,
        column: &ColumnReference,
        spec: &ColumnDefinition,
        additional_data_range: &Range<usize>,
    ) -> RawValue {
        match field_info.storage_type {
            FieldCompressionType::None { .. } => {
                self.encapsulate_raw(values, column, spec)
            }
            FieldCompressionType::Bitpacked { .. } => {
                let bitpacked_data = Self::read_bitpacked(values, field_info);

                self.encapsulate_raw(&bitpacked_data, column, spec)
            }
            FieldCompressionType::Common { .. } => { 
                todo!()
            }
            FieldCompressionType::BitpackedIndexed { .. } => {
                let raw_offsets = Self::read_bitpacked(values, field_info);
                let offsets: &[u32] = cast_slice(&raw_offsets);

                // Shift the pallet data to the range related to this column and
                // cast the pallet data to u32s.
                let pallet_data: &[u32] = cast_slice(&self.pallet[additional_data_range.start .. additional_data_range.end]);

                // For each value in the input slice
                let values : Vec<_> = offsets.iter()
                    .map(|&offset| pallet_data[offset as usize])
                    .collect();

                let byte_slice : &[u8] = cast_slice(&values);
                self.encapsulate_raw(byte_slice, column, spec)
            }
            FieldCompressionType::BitpackedIndexedArray { .. } => {
                // TODO: incorrect! use category_index to determine data range
                let pallet_indices : &[u32] = cast_slice(values);
                assert_eq!(pallet_indices.len(), 4);

                let range = Range {
                    start: pallet_indices[0] as usize,
                    end: (pallet_indices[0] + column.arity) as usize
                };

                let values = &self.pallet[range];
                let byte_slice: &[u8] = cast_slice(&values);
                
                self.encapsulate_raw(byte_slice, column, spec)
            }
        }
    }
}

/// A table stores all values for all columns of all rows.
pub struct Table {
    /// Indexed by row then by column
    records: Vec<Vec<RawValue>>,
}

mod chunks {
    use crate::dbcd::raw::{Chunked, Raw};

    pub enum Content<'a> {
        Regular { records: Chunked<'a>, string_block : Raw<'a> },
        OffsetMap { records: Raw<'a>, offset_map: Chunked<'a> }
    }
}

#[cfg(test)]
pub mod tests {
    use std::io::BufRead;

    use crate::dbcd::dbd::Definition;
    use super::WDC1;

    #[test]
    pub fn test_map() {
        let dbc_data = include_bytes!("../../tests/wdc1/map.db2.wdc1");
        let dbd_data = include_bytes!("../../tests/map.dbd.sample");

        let dbd = Definition::new(dbd_data.lines(), "Map.dbd".to_string()).unwrap();
        let dbc = WDC1::new(&dbc_data[4..]);

        let now = std::time::Instant::now();
        let rows = dbc.parse(&dbd);
        println!("{} rows parsed in {:.3?}", rows.len(), now.elapsed());

    }

    #[test]
    pub fn test_area_table() {
        let dbc_data = include_bytes!("../../tests/wdc1/AreaTable.db2.0CA01129");
        let dbd_data = include_bytes!("../../tests/AreaTable.dbd");

        let dbd = Definition::new(dbd_data.lines(), "AreaTable.dbd".to_string()).unwrap();
        let dbc = WDC1::new(&dbc_data[4..]);

        let now = std::time::Instant::now();
        let rows = dbc.parse(&dbd);
        println!("{} rows parsed in {:.3?}", rows.len(), now.elapsed());

        println!("{:#?}", rows[0]);
    }
}