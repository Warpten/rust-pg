use std::ops::Range;
use bytemuck::cast_slice;
use bytes::Buf;
use chunks::Content;
use smallvec::{smallvec, SmallVec, ToSmallVec};

use crate::dbcd::dbd::{ColumnDefinition, ColumnReference, ColumnType, Definition};
use crate::dbcd::structured::{FieldCompressionType, Relation};
use crate::dbcd::{raw::{Chunked, Raw}, structured::FieldInfo};
use crate::dbcd::raw::{ChunkedBuf, ChunkedTrait, RawTrait};
use crate::dbcd::dbd::{ColumnCategory, StructureDefinition};
use crate::dbcd::raw::RawBuf;
use crate::dbcd::structured::{Common, ExtendedFieldInfo, FieldCompressionCategory};
use crate::dbcd::typed::table::Table;

use super::shared::{RawValue, DBC};

pub struct WDC1<'a> {
    fields: Chunked<'a>,
    records: Content<'a>,
    id_list: RawBuf,
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
    pub fn new(source: &[u8]) -> WDC1 {
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
        let _id_index = cursor.get_u16_le() as usize;
        let total_field_count = cursor.get_u32_le() as usize;
        let _bitpacked_data_offset = cursor.get_u32_le() as usize;
        let _lookup_column_count = cursor.get_u32_le() as usize;
        let offset_map_offset = cursor.get_u32_le() as usize;
        let id_list_size = cursor.get_u32_le() as usize;
        let field_storage_info_size = cursor.get_u32_le() as usize;
        let common_data_size = cursor.get_u32_le() as usize;
        let pallet_data_size = cursor.get_u32_le() as usize;
        let relationship_data_size = cursor.get_u32_le() as usize;

        let (fields, cursor) = Chunked::from_raw(cursor, 2 + 2, total_field_count);

        let (records, cursor) = if (flags & 0x01) == 0 {
            let (records, cursor) = ChunkedBuf::from_raw(cursor, record_size, record_count);
            let (string_block, cursor) = Raw::from_raw(cursor, string_table_size);

            (Content::Regular { records, string_block }, cursor)
        } else {
            let distance = unsafe {
                cursor.as_ptr().offset_from(source.as_ptr()) as usize
            };

            let (records, cursor) = Raw::from_raw(cursor, offset_map_offset - distance);
            let (offset_map, cursor) = Chunked::from_raw(cursor, 4 + 2, max_id - min_id + 1);

            (Content::OffsetMap { records, offset_map }, cursor)
        };

        let (id_list, cursor) = RawBuf::from_raw(cursor, id_list_size);
        let (copy_table, cursor) = Chunked::optionally_from(copy_table_size > 0, cursor, 4 + 4, copy_table_size / 8);
        let (field_info, cursor) = Chunked::from_raw(cursor, 2 + 2 + 4 + 4 + 3 * 4, field_storage_info_size / (2 + 2 + 4 + 4 + 3 * 4));
        let (pallet, cursor) = Raw::from_raw(cursor, pallet_data_size);
        let (common, cursor) = Raw::from_raw(cursor, common_data_size);
        let (relationship, cursor) = Raw::from_raw(cursor, relationship_data_size);

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
}

impl<'a> DBC for WDC1<'a> {
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

pub struct Parser<'a> {
    field_info: Vec<ExtendedFieldInfo>,

    pallet_data: Vec<u8>,
    common_data: Common,
    relation_data: Relation,

    noninline_record_ids: Vec<u8>,
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
        let common = Common::new(data.common.data(), &field_info);

        Parser {
            field_info,
            pallet_data: data.pallet.to_vec(),
            common_data: common,
            relation_data: Relation::new(data.relationship.data()),

            noninline_record_ids: data.id_list.to_vec(),
            records: data.records,
        }
    }

    /// Returns all values for all columns of all rows.
    /// The buffer returned has double indexing based on row index and column index.
    /// The first index is the column index; the second index is the row index.
    /// 
    /// TL;DR: Read with `result[column_index][record_index]`.
    fn parse(&self, definition: &Definition, structure: &StructureDefinition) -> (Vec<Vec<RawValue>>, Vec<u32>) {
        // Collect record data.
        let records: Vec<(&[u8], usize)> = match &self.records {
            Content::Regular { records, .. } => {
                // Effectively just an array of pointers over contiguous memory
                records.iter()
                    .enumerate()
                    .map(|(index, record)| {
                        (record, index * records.stride())
                    })
                    .collect()
            },
            Content::OffsetMap { records, offset_map } => {
                offset_map.materialize(|mut chunk| {
                    let offset = chunk.get_u32_le() as usize;
                    let size = chunk.get_u16_le() as usize;

                    let range = Range {
                        start: offset,
                        end: size + offset
                    };

                    (&records[range], offset)
                }).collect::<Vec<_>>()
            }
        };

        let mut column_categories: [Vec<_>; ColumnCategory::MAX as usize] = std::array::from_fn(|_| Vec::new());
        for column in structure.iter() {
            column_categories[column.category as usize].push(column);
        }

        // Look for the ID column
        // Technically we can just look for an id_list and assume the column is non-inline
        //   but treat DBD as source or truth and follow through it.
        //   Nothing stops Blizzard from doing stupid things and including a noninline ID
        //   list while at the same time having the real ID inline.
        let (ids, injection_point) = column_categories[ColumnCategory::Inlined as usize]
            .iter()
            .position(|c| c.id)
            .map(|idx| {
                let col = &structure[idx];
                let spec = unsafe {
                    // SAFETY: this should never fail; the file is malformed otherwise.
                    definition.spec(col).unwrap_unchecked()
                };
                let efi = &self.field_info[idx];

                let ids: Vec<_> = records.iter().map(|&(rec, _)| {
                    self.parse_value(0, rec, efi, col, spec, |mut bytes, item_width| {
                        assert_eq!(item_width, 4);

                        bytes.get_u32_le()
                    })
                }).collect();

                (ids, usize::MAX)
            })
            .unwrap_or_else(|| {
                assert_eq!(column_categories[ColumnCategory::ID as usize].len(), 1);

                let ids = cast_slice(&self.noninline_record_ids).to_vec();

                (ids, column_categories[ColumnCategory::ID as usize][0].index)
            });

        let mut rows: Vec<_> = (0..records.len()).map(|rec_idx| {
            let (rec, rec_ofs) = &records[rec_idx];
            let rec_id = ids[rec_idx];

            let mut values: Vec<_> = column_categories[ColumnCategory::Inlined as usize]
                .iter()
                .enumerate()
                .map(|(col_idx, &col)| {
                    let spec = unsafe {
                        // SAFETY: this should never fail; the file is malformed otherwise.
                        definition.spec(col).unwrap_unchecked()
                    };
                    let efi = &self.field_info[col_idx];

                    self.parse_value(rec_id, &rec, efi, col, spec, |bytes, item_width| {
                        self.encapsulate_raw(bytes, item_width, col, spec)
                    })
                })
                .collect();

            if let Some(relation) = self.relation_data.get(rec_ofs) {
                values.push(RawValue::ForeignKey(*relation));
            }

            values
        }).collect();

        if injection_point != usize::MAX {
            let id_count = ids.len();
            assert_eq!(id_count, records.len());

            rows.iter_mut()
                .enumerate()
                .for_each(|(row_idx, row)| {
                    // Deviate from DBD spec: not all IDs are u32; treat everything as U32.
                    row.insert(injection_point, RawValue::U32(smallvec![ids[row_idx]]));
                });
        }

        (rows, ids)
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
    pub fn parse_value<C, R>(&self,
        id: u32,
        record: &[u8],
        field_info: &ExtendedFieldInfo,
        column: &ColumnReference,
        spec: &ColumnDefinition,
        consumer: C
    ) -> R
        where C: Fn(&[u8], usize) -> R
    {
        assert_eq!(field_info.field.arity(), column.arity, "{:#?}", (column, field_info, spec));

        let byte_offset = field_info.field.offset_bytes();
        let byte_width = field_info.field.size_bytes();
        assert_ne!(byte_width, 0, "Impossible field width {:#?}", field_info);

        let range = Range {
            start: byte_offset,
            end: byte_offset + byte_width
        };

        let raw_bytes = &record[range];
        self.transform_values_raw(id, raw_bytes, field_info, spec, consumer)
    }

    fn encapsulate_raw(&self, values: &[u8], item_width: usize, column: &ColumnReference, spec: &ColumnDefinition) -> RawValue {
        match &spec.column_type {
            // LocalizedString sometimes implies [16] but there's no way to know that
            // so just throw hands and pretend it's a string.
            ColumnType::String | ColumnType::LocalizedString => {
                match &self.records {
                    Content::Regular { string_block, .. } => {
                        assert_eq!(item_width, std::mem::size_of::<u32>());

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
                        let slices = values.split(|&c| c == 0)
                            .map(|slice| unsafe {
                                String::from_utf8_unchecked(slice.to_vec())
                            });

                        RawValue::String(SmallVec::from_iter(slices))
                    },
                }
            },
            ColumnType::Integer => {
                let step = column.bits.unwrap_or(32);
                match (step, column.unsigned) {
                    (0..=8,  false) => RawValue::I8 (SmallVec::from_slice(
                        cast_slice::<_, i8>(&values[..column.arity])
                    )),
                    (9..=16, false) => {
                        let expanded = values[..column.arity * item_width].chunks(item_width).map(|mut c| {
                            c.get_uint_le(item_width) as _
                        });
                        RawValue::I16(SmallVec::from_iter(expanded))
                    },
                    (0..=32, false) => {
                        let expanded = values[..column.arity * item_width].chunks(item_width).map(|mut c| {
                            c.get_uint_le(item_width) as _
                        });
                        RawValue::I32(SmallVec::from_iter(expanded))
                    },
                    (0..=64, false) => {
                        let expanded = values[..column.arity * item_width].chunks(item_width).map(|mut c| {
                            c.get_uint_le(item_width) as _
                        });
                        RawValue::I64(SmallVec::from_iter(expanded))
                    },
                    (0..=8,  true)  => RawValue::U8 (SmallVec::from_slice(&values[..column.arity])),
                    (9..=16, true)  => {
                        let expanded = values[..column.arity * item_width].chunks(item_width).map(|mut c| {
                            c.get_uint_le(item_width) as _
                        });
                        RawValue::U16(SmallVec::from_iter(expanded))
                    },
                    (0..=32, true)  => {
                        let expanded = values[..column.arity * item_width].chunks(item_width).map(|mut c| {
                            c.get_uint_le(item_width) as _
                        });
                        RawValue::U32(SmallVec::from_iter(expanded))
                    },
                    (0..=64, true)  => {
                        let expanded = values[..column.arity * item_width].chunks(item_width).map(|mut c| {
                            c.get_uint_le(item_width) as _
                        });
                        RawValue::U64(SmallVec::from_iter(expanded))
                    },
                    (bits, _) => panic!("Unsupported {} bit width", bits)
                }
            },
            ColumnType::Float => {
                assert_eq!(item_width, std::mem::size_of::<f32>());

                RawValue::F32(cast_slice(values).to_smallvec())
            },
        }
    }

    fn read_bitpacked(values: &[u8], field_info: &ExtendedFieldInfo) -> Vec<u64> {
        // At most, 64 bits, aka 8 bytes
        let byte_width = field_info.field.size_bytes();
        let shift_offset = 64 - field_info.field.size_bits() - (field_info.field.offset_bits() % 8);
        let mask_offset = 64 - field_info.field.size_bits();

        let mut cursor = values;

        let mut result = Vec::with_capacity(values.len());

        for _ in 0..(values.len() / byte_width) {
            let raw_integer = cursor.get_uint_le(byte_width);
            let value = (raw_integer << shift_offset) >> mask_offset;
            result.push(value);
        }

        result
    }

    fn transform_values_raw<C, R>(&self,
        id: u32,
        values: &[u8],
        field_info: &ExtendedFieldInfo,
        spec: &ColumnDefinition,
        consumer: C
    ) -> R
        where C : Fn(&[u8], usize) -> R
    {
        match field_info.field.storage_type {
            FieldCompressionType::None { .. } => {
                consumer(values, field_info.field.element_bytes())
            }
            FieldCompressionType::Bitpacked { .. } => {
                let bitpacked_data = Self::read_bitpacked(values, field_info);
                let raw_bytes: &[u8] = cast_slice(&bitpacked_data);

                consumer(raw_bytes, 8)
            }
            FieldCompressionType::Common { default, .. } => {
                assert!(matches!(spec.column_type, ColumnType::Integer | ColumnType::Float),
                    "Unexpected compression type for non-arithmetic field {:#?}", field_info.field.storage_type);

                // Common data is aligned to 4 bytes in WDC1 meaning we can just pretend the values read were from
                // 4 bytes and casts will handle the rest.
                let value = &[self.common_data.read(field_info.category_index, id, default)];
                let raw_bytes: &[u8] = cast_slice(value);

                consumer(raw_bytes, 4)
            }
            FieldCompressionType::BitpackedIndexed { .. } => {
                let bitpacked_data = Self::read_bitpacked(values, field_info);

                // Shift the pallet data to the range related to this column and
                // cast the pallet data to u32s.
                let pallet_data: &[u32] = cast_slice(&self.pallet_data[field_info.additional_data_range.clone()]);

                // For each value in the input slice
                let values : Vec<_> = bitpacked_data.iter()
                    .map(|&offset| pallet_data[offset as usize])
                    .collect();

                let raw_bytes: &[u8] = cast_slice(&values);
                consumer(raw_bytes, 4) // Pallet stores U32s
            }
            FieldCompressionType::BitpackedIndexedArray { arity, .. } => {
                let raw_offsets = Self::read_bitpacked(values, field_info);
                assert_eq!(raw_offsets.len(), 1, "Potentially invalid arity for bitpacked indexed pallet data: {:#?}", field_info);

                let arity = arity as u64;
                let range = Range {
                    start: field_info.additional_data_range.start + (raw_offsets[0] * arity) as usize * 4,
                    end:   field_info.additional_data_range.start + ((raw_offsets[0] + 1) * arity) as usize * 4
                };

                // Shift the pallet data to the range related to this column and
                // cast the pallet data to u32s.
                let values: &[u32] = cast_slice(&self.pallet_data[range]);
                let byte_slice: &[u8] = cast_slice(&values);

                consumer(byte_slice, 4)
            }
        }
    }
}

mod chunks {
    use crate::dbcd::raw::{Chunked, ChunkedBuf, Raw};

    pub enum Content<'a> {
        Regular { records: ChunkedBuf, string_block : Raw<'a> },
        OffsetMap { records: Raw<'a>, offset_map: Chunked<'a> }
    }
}

#[cfg(test)]
pub mod tests {
    use std::io::BufRead;

    use crate::dbcd::{dbc::shared::{RawValue, DBC}, dbd::Definition};
    use super::WDC1;

    macro_rules! validate_column {
        ($row:expr, $spec:expr, $col:literal, $pattern:path, $value:expr) => {
            match &$row[$spec[$col].index] {
                $pattern(ref v) if v.iter().zip($value.iter()).filter(|(l, r)| l != r).count() == 0 => (),
                actual => panic!("An error occured while parsing '{}': expected {:?}, found {:?}", $col, $value, actual)
            }
        }
    }

    #[test]
    pub fn test_map() {
        let dbc_data = include_bytes!("../../../tests/wdc1/map.db2.wdc1");
        let dbd_data = include_bytes!("../../../tests/map.dbd.sample");

        let dbd = Definition::new(dbd_data.lines(), "Map.dbd".to_string()).unwrap();
        let dbc = WDC1::new(&dbc_data[4..]);

        let now = std::time::Instant::now();
        let rows = dbc.parse(&dbd);
        println!("{} rows parsed in {:.3?}", rows.len(), now.elapsed());
    }

    #[test]
    pub fn test_table() {
        let dbc_data = include_bytes!("../../../tests/wdc1/CreatureDisplayInfo.db2.406268DF");
        let dbd_data = include_bytes!("../../../tests/CreatureDisplayInfo.dbd");

        let dbd = Definition::new(dbd_data.lines(), "CreatureDisplayInfo.dbd".to_string()).unwrap();
        let dbc = WDC1::new(&dbc_data[4..]);
        let spec = dbd.select(Some(dbc.layout_hash), None).unwrap();

        let now = std::time::Instant::now();
        let table = dbc.parse_table(&dbd).unwrap();
        println!("{} rows parsed in {:.3?} ({} columns)", table.len(), now.elapsed(), table.column_count());
        
        let entry = table.get(30).unwrap();
        validate_column!(entry, spec, "ID",                            RawValue::I32, &[30]);
        validate_column!(entry, spec, "CreatureModelScale",            RawValue::F32, &[0.4]);
        validate_column!(entry, spec, "ModelID",                       RawValue::U16, &[30]);
        validate_column!(entry, spec, "NPCSoundID",                    RawValue::U16, &[0]);
        validate_column!(entry, spec, "SizeClass",                     RawValue::I8,  &[1]);
        validate_column!(entry, spec, "Flags",                         RawValue::U8,  &[0x0]);
        validate_column!(entry, spec, "Gender",                        RawValue::I8,  &[2]);
        validate_column!(entry, spec, "ExtendedDisplayInfoID",         RawValue::I32, &[0]);
        validate_column!(entry, spec, "PortraitTextureFileDataID",     RawValue::I32, &[0]);
        validate_column!(entry, spec, "CreatureModelAlpha",            RawValue::U8,  &[255]);
        validate_column!(entry, spec, "SoundID",                       RawValue::U16, &[0]);
        validate_column!(entry, spec, "PlayerOverrideScale",           RawValue::F32, &[0.0]);
        validate_column!(entry, spec, "PortraitCreatureDisplayInfoID", RawValue::I32, &[0]);
        validate_column!(entry, spec, "BloodID",                       RawValue::U8,  &[0]);
        validate_column!(entry, spec, "ParticleColorID",               RawValue::U16, &[0]);
        validate_column!(entry, spec, "CreatureGeosetData",            RawValue::I32, &[0]);
        validate_column!(entry, spec, "ObjectEffectPackageID",         RawValue::U16, &[0]);
        validate_column!(entry, spec, "AnimReplacementSetID",          RawValue::U16, &[0]);
        validate_column!(entry, spec, "UnarmedWeaponType",             RawValue::I8,  &[-1]);
        validate_column!(entry, spec, "StateSpellVisualKitID",         RawValue::I32, &[0]);
        validate_column!(entry, spec, "PetInstanceScale",              RawValue::F32, &[1.0]);
        validate_column!(entry, spec, "MountPoofSpellVisualKitID",     RawValue::I32, &[0]);
        validate_column!(entry, spec, "TextureVariationFileDataID",    RawValue::I32, &[124911, 0, 0]);
    }

    #[test]
    pub fn test_creature_display_info() {
        let dbc_data = include_bytes!("../../../tests/wdc1/CreatureDisplayInfo.db2.406268DF");
        let dbd_data = include_bytes!("../../../tests/CreatureDisplayInfo.dbd");

        let dbd = Definition::new(dbd_data.lines(), "CreatureDisplayInfo.dbd".to_string()).unwrap();
        let dbc = WDC1::new(&dbc_data[4..]);
        let spec = dbd.select(Some(dbc.layout_hash), None).unwrap();

        let now = std::time::Instant::now();
        let rows = dbc.parse(&dbd);
        println!("{} rows parsed in {:.3?}", rows.len(), now.elapsed());

        let entry = &rows[13];
        validate_column!(entry, spec, "ID",                            RawValue::I32, &[30]);
        validate_column!(entry, spec, "CreatureModelScale",            RawValue::F32, &[0.4]);
        validate_column!(entry, spec, "ModelID",                       RawValue::U16, &[30]);
        validate_column!(entry, spec, "NPCSoundID",                    RawValue::U16, &[0]);
        validate_column!(entry, spec, "SizeClass",                     RawValue::I8,  &[1]);
        validate_column!(entry, spec, "Flags",                         RawValue::U8,  &[0x0]);
        validate_column!(entry, spec, "Gender",                        RawValue::I8,  &[2]);
        validate_column!(entry, spec, "ExtendedDisplayInfoID",         RawValue::I32, &[0]);
        validate_column!(entry, spec, "PortraitTextureFileDataID",     RawValue::I32, &[0]);
        validate_column!(entry, spec, "CreatureModelAlpha",            RawValue::U8,  &[255]);
        validate_column!(entry, spec, "SoundID",                       RawValue::U16, &[0]);
        validate_column!(entry, spec, "PlayerOverrideScale",           RawValue::F32, &[0.0]);
        validate_column!(entry, spec, "PortraitCreatureDisplayInfoID", RawValue::I32, &[0]);
        validate_column!(entry, spec, "BloodID",                       RawValue::U8,  &[0]);
        validate_column!(entry, spec, "ParticleColorID",               RawValue::U16, &[0]);
        validate_column!(entry, spec, "CreatureGeosetData",            RawValue::I32, &[0]);
        validate_column!(entry, spec, "ObjectEffectPackageID",         RawValue::U16, &[0]);
        validate_column!(entry, spec, "AnimReplacementSetID",          RawValue::U16, &[0]);
        validate_column!(entry, spec, "UnarmedWeaponType",             RawValue::I8,  &[-1]);
        validate_column!(entry, spec, "StateSpellVisualKitID",         RawValue::I32, &[0]);
        validate_column!(entry, spec, "PetInstanceScale",              RawValue::F32, &[1.0]);
        validate_column!(entry, spec, "MountPoofSpellVisualKitID",     RawValue::I32, &[0]);
        validate_column!(entry, spec, "TextureVariationFileDataID",    RawValue::I32, &[124911, 0, 0]);
    }

    #[test]
    pub fn test_area_table() {
        // Note: in my data set this dbc has been modified so that the last column of DunMorogh is set to 1
        let dbc_data = include_bytes!("../../../tests/wdc1/AreaTable.db2.0CA01129");
        let dbd_data = include_bytes!("../../../tests/AreaTable.dbd");

        let dbd = Definition::new(dbd_data.lines(), "AreaTable.dbd".to_string()).unwrap();
        let dbc = WDC1::new(&dbc_data[4..]);
        let spec = dbd.select(Some(dbc.layout_hash), None).unwrap();

        let now = std::time::Instant::now();
        let rows = dbc.parse(&dbd);
        println!("{} rows parsed in {:.3?}", rows.len(), now.elapsed());

        let dun_morogh = &rows[0];
        validate_column!(dun_morogh, spec, "ID",                 RawValue::U32,    &[1]);
        validate_column!(dun_morogh, spec, "ZoneName",           RawValue::String, &["DunMorogh"]);
        validate_column!(dun_morogh, spec, "AreaName_lang",      RawValue::String, &["Dun Morogh"]);
        validate_column!(dun_morogh, spec, "Flags",              RawValue::I32,    &[0x4041, 0]);
        validate_column!(dun_morogh, spec, "Ambient_multiplier", RawValue::F32,    &[0.3]);
        validate_column!(dun_morogh, spec, "ContinentID",        RawValue::U16,    &[0]);
        validate_column!(dun_morogh, spec, "ParentAreaID",       RawValue::U16,    &[0]);
        validate_column!(dun_morogh, spec, "AreaBit",            RawValue::I16,    &[119]);
        validate_column!(dun_morogh, spec, "AmbienceID",         RawValue::U16,    &[598]);
        validate_column!(dun_morogh, spec, "ZoneMusic",          RawValue::U16,    &[759]);
        validate_column!(dun_morogh, spec, "IntroSound",         RawValue::U16,    &[0]);
        validate_column!(dun_morogh, spec, "LiquidTypeID",       RawValue::U16,    &[0, 0, 0, 0]);
        validate_column!(dun_morogh, spec, "UwZoneMusic",        RawValue::U16,    &[0]);
        validate_column!(dun_morogh, spec, "UwAmbience",         RawValue::U16,    &[675]);
        validate_column!(dun_morogh, spec, "PvpCombatWorldStateID", RawValue::I16, &[-1]);
        validate_column!(dun_morogh, spec, "SoundProviderPref",  RawValue::U8,     &[0]);
        validate_column!(dun_morogh, spec, "SoundProviderPrefUnderwater", RawValue::U8, &[11]);
        validate_column!(dun_morogh, spec, "ExplorationLevel",   RawValue::I8,     &[0]);
        validate_column!(dun_morogh, spec, "FactionGroupMask",   RawValue::U8,     &[2]);
        validate_column!(dun_morogh, spec, "MountFlags",         RawValue::U8,     &[15]);
        validate_column!(dun_morogh, spec, "WildBattlePetLevelMin", RawValue::U8,  &[1]);
        validate_column!(dun_morogh, spec, "WildBattlePetLevelMax", RawValue::U8,  &[2]);
        validate_column!(dun_morogh, spec, "WindSettingsID",     RawValue::U8,     &[0]);
        validate_column!(dun_morogh, spec, "UwIntroSound",       RawValue::U32,    &[1]);
    }
}