use std::{collections::HashMap, ops::Range};

use bytes::Buf;
use custom_attrs::CustomAttrs;
use crate::dbcd::structured::{FieldCompressionType, FieldInfo};

use super::ExtendedFieldInfo;

pub struct Common {
    entries: Vec<CommonEntry>,
}
impl Common {
    pub fn new(data: &[u8], fields: &[ExtendedFieldInfo]) -> Self {
        let mut entries = vec![];
        for field in fields {
            if matches!(field.field.storage_type, FieldCompressionType::Common { .. }) {
                let chunk = &data[field.additional_data_range.clone()];

                let values: HashMap<_, _> = chunk.chunks(8).map(|mut c| {
                    let id = c.get_u32_le();
                    let value = c.get_u32_le();
                    (id, value)
                }).collect();

                let entry = CommonEntry {
                    value_type: CommonValueType::Padded,
                    values,
                };
                entries.push(entry);
            }
        }

        Self { entries }
    }
    pub fn from_legacy(mut data: &[u8], padded: bool) -> Self {
        if !data.has_remaining() {
            return Self {
                entries: vec![]
            };
        }

        let num_columns = data.get_u32_le();
        let mut entries = vec![];

        for _ in 0..num_columns {
            let cnt = data.get_u32_le() as usize;
            let value_type = unsafe {
                std::mem::transmute::<_, CommonValueType>(data.get_u8())
            };

            let item_size = if padded {
                4
            } else {
                value_type.width()
            };

            let (chunk, remainder) = data.split_at((4 + item_size) * cnt);
            data = remainder;

            let values: HashMap<_, _> = chunk.chunks(4 + item_size).map(|mut element| {
                let id = element.get_u32_le();
                let data = element.get_uint_le(item_size) as u32;

                (id, data)
            }).collect();

            entries.push(CommonEntry {
                value_type,
                values,
            });
        }

        Self {
            entries,
        }
    }

    pub fn read(&self, column_index: usize, id: u32, default_value: u32) -> u32 {
        let entry = &self.entries[column_index];
        entry.values.get(&id).copied().unwrap_or(default_value)
    }
}

pub struct CommonEntry {
    value_type: CommonValueType,
    values: HashMap<u32, u32 /* raw bytes */>,
}

#[repr(u8)]
#[derive(CustomAttrs)]
#[attr(
    #[function = "width"]
    pub width: usize
)]
pub enum CommonValueType {
    #[attr(width = std::mem::size_of::<u32>())]
    String = 0,
    #[attr(width = std::mem::size_of::<u16>())]
    Short = 1,
    #[attr(width = std::mem::size_of::<u8>())]
    Byte = 2,
    #[attr(width = std::mem::size_of::<f32>())]
    Float = 3,
    #[attr(width = std::mem::size_of::<u32>())]
    Integer = 4,
    #[attr(width = std::mem::size_of::<u64>())]
    Long = 5,

    #[attr(width = std::mem::size_of::<u32>())]
    Padded = 6,
}