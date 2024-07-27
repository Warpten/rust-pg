use std::{ops::{Index, Range}, slice::Chunks};

#[macro_export]
macro_rules! chunk {
    ($t:tt, Chunked) => {
        pub struct $t<'a> {
            chunk: &'a[u8],
            stride: usize
        }
        impl<'a> $t<'a> {
            #[inline]
            pub fn read(stride: usize, count: usize, cursor: &'a [u8]) -> (Self, &'a[u8]) {
                let (chunk, remainder) = cursor.split_at(stride * count);
                (Self { chunk, stride }, remainder)
            }

            #[inline]
            pub fn read_optional(flag: bool, stride: usize, count: usize, mut cursor: &'a [u8]) -> (Option<Self>, &'a [u8]) {
                if flag {
                    let (slf, remainder) = Self::read(stride, count, cursor);
                    (Some(slf), remainder)
                } else {
                    (None, cursor)
                }
            }

            pub fn chunks(&self) -> std::slice::Chunks<'a, u8> {
                self.chunk.chunks(self.stride)
            }

            pub fn byte_len(&self) -> usize { self.chunk.len() }
        }
    };
    ($t:tt, Raw) => {
        pub struct $t<'a>(&'a[u8]);
        impl<'a> $t<'a> {
            #[inline]
            pub fn read(length: usize, cursor: &'a[u8]) -> (Self, &'a[u8]) {
                let (chunk, remainder) = cursor.split_at(length);
                (Self(chunk), remainder)
            }

            #[inline]
            pub fn read_optional(flag: bool, length: usize, mut cursor: &'a [u8]) -> (Option<Self>, &'a [u8]) {
                if flag {
                    let (slf, remainder) = Self::read(length, cursor);
                    (Some(slf), remainder)
                } else {
                    (None, cursor)
                }
            }

            pub fn byte_len(&self) -> usize { self.0.len() }
        }
    };
}

chunk! { Chunked, Chunked }
chunk! { Raw, Raw }

pub struct StringBlock<'a> {
    data: &'a[u8],
    offsets: Vec<usize>
}
impl<'a> StringBlock<'a> {
    pub fn read(sz: usize, data: &'a[u8]) -> (Self, &'a[u8]) {
        let (block, remainder) = data.split_at(sz);

        let offsets : Vec<_> = block.into_iter()
            .enumerate()
            .filter(|&(_, b)| *b == 0)
            .map(|(i, _)| i)
            .collect();

        (Self { data, offsets }, remainder)
    }

    pub fn get(&self, idx: usize) -> Option<&[u8]> {
        match self.offsets.get(idx) {
            Some(&offset) => {
                let end = self.offsets.get(idx + 1).copied().unwrap_or(self.data.len() + 1);

                Some(&self.data[offset..end - 1])
            },
            None => None
        }
    }
}