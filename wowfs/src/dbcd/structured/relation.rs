use std::collections::HashMap;
use std::ops::Deref;
use bytemuck::cast_slice;
use bytes::Buf;

pub struct Relation {
    data: HashMap<usize /* record offset*/, u32 /* column value*/>,
}

impl Deref for Relation {
    type Target = HashMap<usize, u32>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl Relation {
    pub fn new(mut data: &[u8]) -> Self {
        if data.remaining() > 12 {
            let num_bytes = data.get_u32_le() as usize * 8;
            let _min_id = data.get_u32_le();
            let _max_id = data.get_u32_le();

            if data.remaining() != num_bytes {
                Self { data: HashMap::new() }
            } else {
                let entries: &[u32] = cast_slice(&data[..num_bytes]);
                let data = entries.chunks(2)
                    .into_iter()
                    .map(|data| (data[1] as usize, data[0]))
                    .collect();
                Self { data }
            }
        } else {
            Self { data: HashMap::new() }
        }
    }
}