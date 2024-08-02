use std::ops::Range;

use bytes::Buf;
use custom_attrs::CustomAttrs;

pub struct Common {
    data: Vec<u8>, // Contains all data for all columns, concatenated
    entries: Vec<CommonEntry>,
}
impl Common {
    pub fn new(mut data: &[u8], padded: bool) -> Self {
        if !data.has_remaining() {
            return Self {
                data: vec![],
                entries: vec![]
            };
        }

        let num_columns = data.get_u32_le();
        let mut flattened_data = vec![];
        let mut entries = vec![];

        for _ in 0..num_columns {
            let cnt = data.get_u32_le() as usize;
            let value_type = unsafe {
                std::mem::transmute::<_, CommonValueType>(data.get_u8())
            };

            let split_offset = if padded {
                cnt * (4 + 4)
            } else {
                cnt * (4 + value_type.width())
            };

            let (chunk, remainder) = data.split_at(split_offset);
            data = remainder;

            let range = Range {
                start: flattened_data.len(),
                end: flattened_data.len() + split_offset
            };

            flattened_data.extend_from_slice(chunk);
            entries.push(CommonEntry {
                value_type,
                range,
            });
        }

        Self {
            data: flattened_data,
            entries,
        }
    }

    pub fn read(&self, column_index: usize, offset: usize) -> &[u8] {
        let entry = &self.entries[column_index];
        let range = &self.data[entry.range.clone()];

        &range[offset..(offset + entry.value_type.width())]
    }
}

pub struct CommonEntry {
    value_type: CommonValueType,
    range: Range<usize>,
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
    Long = 5
}