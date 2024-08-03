use std::io::Read;
use std::ops::{Deref, Range};
use bytemuck::{cast_slice, AnyBitPattern, NoUninit};
use bytes::Buf;
use smallvec::{smallvec, SmallVec, ToSmallVec};

use crate::dbcd::dbd::{ColumnDefinition, ColumnReference, ColumnType, Definition};

use crate::dbcd::structured::FieldCompressionType;
use crate::dbcd::{raw::{Chunked, Raw}, structured::FieldInfo};
use crate::dbcd::raw::{ChunkedBuf, ChunkedTrait, RawTrait};
use crate::dbcd::wdc1::chunks::Content;

use super::dbd::StructureDefinition;
use super::raw::RawBuf;
use super::shared::{self, Parser as ParserTrait, RawValue};
use super::structured::{Common, ExtendedFieldInfo, FieldCompressionCategory};

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

impl<'a> shared::Parsable for WDC1<'a> {
    fn parse_with(self, definition: &Definition, spec: &StructureDefinition) -> Vec<Vec<RawValue>> {
        Parser::new(self).parse(definition, spec)
    }

    fn select_structure(&self, definition: &Definition) -> Option<&StructureDefinition> {
        definition.select(Some(self.layout_hash), None)
    }
}

pub struct Parser<'a> {
    field_info: Vec<ExtendedFieldInfo>,
    pallet: Vec<u8>,
    common: Common,
    ids: Vec<u8>,

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
            pallet: data.pallet.to_vec(),
            ids: data.id_list.to_vec(),
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
        // Collect record data.
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

        // Collect the ID column first
        let (ids, inject) = structure.iter()
            .find(|column| column.id)
            .map(|column| {
                // Fast path if non-inline
                if column.noninline {
                    (cast_slice::<_, u32>(&self.ids).to_vec(), column.index as isize)
                } else {
                    // ...Find the specification
                    let spec = unsafe {
                        // SAFETY: this should never fail; the file is malformed otherwise.
                        definition.spec(column).unwrap_unchecked()
                    };

                    let field_info = &self.field_info[0];
                    assert_ne!(field_info.field.storage_type.category(), FieldCompressionCategory::Common);

                    // ...For each record, read the column's values
                    let mut values = vec![];
                    for record in &records {
                        // Ignore the ID passed as argument here - it's only used for common values, which this
                        // field cannot be
                        values.push(self.parse_value(0, record, field_info, column, spec, |bytes, item_width| {
                            assert_eq!(item_width, 4);

                            let mut cursor = bytes;
                            cursor.get_u32_le()
                        }));
                    }
                    values.shrink_to_fit();
                    (values, -1)
                }
            })
            .unwrap();

        #[cfg(not(feature = "horizontal_dbc"))]
        {
            let mut rows: Vec<_> = {
                let columns: Vec<_> = structure.iter()
                    .filter(|c| !c.noninline)
                    .enumerate()
                    .map(|(column_index, column_reference)| {
                        // ...Find the specification
                        let spec = unsafe {
                            // SAFETY: this should never fail; the file is malformed otherwise.
                            definition.spec(column_reference).unwrap_unchecked()
                        };

                        let field_info = &self.field_info[column_index];

                        (0..records.len()).map(|record_index| {
                            let record = &records[record_index];
                            let id = ids[record_index];

                            self.parse_value(id, record, field_info, column_reference, spec, |bytes, item_width| {
                                self.encapsulate_raw(bytes, item_width, column_reference, spec)
                            })
                        }).collect::<Vec<_>>()
                    }).collect();

                // Transpose now
                let len = columns[0].len();
                let mut iters: Vec<_> = columns.into_iter().map(|n| n.into_iter()).collect();
                (0..len)
                    .map(|_| {
                        iters
                            .iter_mut()
                            .map(|n| n.next().unwrap())
                            .collect::<Vec<_>>()
                    })
                    .collect()
            };
            if inject >= 0 {
                for i in 0..ids.len() {
                    rows[i].insert(inject as usize, RawValue::U32(smallvec![ids[i]]));
                }
            }
            rows
        }

        #[cfg(feature = "horizontal_dbc")]
        {
            let mut rows: Vec<_> = (0..records.len()).map(|record_index| {
                let record = &records[record_index];
                let id = ids[record_index];

                let mut row: Vec<_> = structure.iter()
                    .filter(|c| !c.noninline)
                    .enumerate()
                    .map(|(column_index, column_reference)| {
                        // ...Find the specification
                        let spec = unsafe {
                            // SAFETY: this should never fail; the file is malformed otherwise.
                            definition.spec(column_reference).unwrap_unchecked()
                        };

                        let field_info = &self.field_info[column_index];

                        self.parse_value(id, record, field_info, column_reference, spec, |bytes, item_width| {
                            self.encapsulate_raw(bytes, item_width, column_reference, spec)
                        })
                    })
                    .collect();

                if inject >= 0 {
                    row.insert(inject as usize, RawValue::U32(smallvec![id]));
                }
                row
            }).collect();

            rows.shrink_to_fit();
            rows
        }
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
                let step = column.bits.unwrap_or(32);
                match (step, column.unsigned) {
                    (0..=8,  false) => RawValue::I8 (SmallVec::from_iter(values.into_iter().map(|&i| i as i8).take(column.arity as _))),
                    (9..=16, false) => {
                        let expanded = values.chunks(item_width).take(column.arity as _).map(|mut data| {
                            data.get_uint_le(item_width) as i16
                        });
                        RawValue::I16(SmallVec::from_iter(expanded))
                    },
                    (0..=32, false) => {
                        let expanded = values.chunks(item_width).take(column.arity as _).map(|mut data| {
                            data.get_uint_le(item_width) as i32
                        });
                        RawValue::I32(SmallVec::from_iter(expanded))
                    },
                    (0..=64, false) => {
                        let expanded = values.chunks(item_width).take(column.arity as _).map(|mut data| {
                            data.get_uint_le(item_width) as i64
                        });
                        RawValue::I64(SmallVec::from_iter(expanded))
                    },
                    (0..=8,  true)  => RawValue::U8 (SmallVec::from_iter(values.into_iter().copied().take(column.arity as _))),
                    (9..=16, true)  => {
                        let expanded = values.chunks(item_width).take(column.arity as _).map(|mut data| {
                            data.get_uint_le(item_width) as u16
                        });
                        RawValue::U16(SmallVec::from_iter(expanded))
                    },
                    (0..=32, true)  => {
                        let expanded = values.chunks(item_width).take(column.arity as _).map(|mut data| {
                            data.get_uint_le(item_width) as u32
                        });
                        RawValue::U32(SmallVec::from_iter(expanded))
                    },
                    (0..=64, true)  => {
                        let expanded = values.chunks(item_width).take(column.arity as _).map(|mut data| {
                            data.get_uint_le(item_width) as u64
                        });
                        RawValue::U64(SmallVec::from_iter(expanded))
                    },
                    (bits, _) => panic!("Unsupported {} bit width", bits)
                }
            },
            ColumnType::Float => {
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
                let value = &[self.common.read(field_info.category_index, id, default)];
                let raw_bytes: &[u8] = cast_slice(value);

                consumer(raw_bytes, 4)
            }
            FieldCompressionType::BitpackedIndexed { .. } => {
                let bitpacked_data = Self::read_bitpacked(values, field_info);

                // Shift the pallet data to the range related to this column and
                // cast the pallet data to u32s.
                let pallet_data: &[u32] = cast_slice(&self.pallet[field_info.additional_data_range.clone()]);

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
                let values: &[u32] = cast_slice(&self.pallet[range]);
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

    use crate::dbcd::{dbd::Definition, shared::RawValue};
    use crate::dbcd::shared::Parsable;
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
        let dbc_data = include_bytes!("../../tests/wdc1/map.db2.wdc1");
        let dbd_data = include_bytes!("../../tests/map.dbd.sample");

        let dbd = Definition::new(dbd_data.lines(), "Map.dbd".to_string()).unwrap();
        let dbc = WDC1::new(&dbc_data[4..]);

        let now = std::time::Instant::now();
        let rows = dbc.parse(&dbd);
        println!("{} rows parsed in {:.3?}", rows.len(), now.elapsed());
    }

    #[test]
    pub fn test_creature_display_info() {
        let dbc_data = include_bytes!("../../tests/wdc1/CreatureDisplayInfo.db2.406268DF");
        let dbd_data = include_bytes!("../../tests/CreatureDisplayInfo.dbd");

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
        let dbc_data = include_bytes!("../../tests/wdc1/AreaTable.db2.0CA01129");
        let dbd_data = include_bytes!("../../tests/AreaTable.dbd");

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