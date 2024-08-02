use std::{iter::Map, ops::Range, slice::Chunks};

/// A chunked chunk effectively slices its content in subelements with the given `stride`.
pub struct Chunked<'a> {
    data: &'a [u8],
    stride: usize,
}
impl<'a> Chunked<'a> {
    /// Constructs a new instanceof this type and returns the remainder of the input data.
    /// 
    /// # Arguments
    /// 
    /// * `input` - An input slice of bytes.
    /// * `stride` - The stride (size) of each sub-element.
    /// * `count` - The amount of sub-elements to encapsulate.
    /// 
    /// # Panics
    ///
    /// Panics if `stride * count > input.len()` or `stride == 0`.
    pub fn from(input: &'a [u8], stride: usize, count: usize) -> (Self, &'a [u8]) {
        let (data, remainder) = input.split_at(stride * count);
        (Chunked { data, stride }, remainder)
    }

    /// Optionally constructs a new instance of this type if the given flag is `true`.
    /// 
    /// # Arguments
    /// 
    /// * `flag` - A flag indicating if data should be read.
    /// * `input` - An input slice of bytes.
    /// * `stride` - The stride (size) of each sub-element.
    /// * `count` - The amount of sub-elements to encapsulate.
    /// 
    /// # Returns
    /// 
    /// Returns a new instance that may be empty.
    /// 
    /// # Panics
    ///
    /// Panics if `stride * count > input.len()` or `stride == 0`.
    #[inline]
    pub fn optionally_from(flag: bool, input: &'a [u8], stride: usize, count: usize) -> (Self, &'a [u8]) {
        static EMPTY : &'static[u8] = &[];

        if flag {
            Self::from(input, stride, count)
        } else {
            (Self { data: EMPTY, stride }, input)
        }
    }

    /// Parses each of the subelements of this chunk's stride through the provided transformation
    /// operation.
    /// 
    /// # Arguments
    /// 
    /// * `transform` - A lambda that transforms a slice of bytes.
    pub fn materialize<T, F>(&self, transform : F) -> Map<Chunks<'_, u8>, F>
        where F : Fn(&'_ [u8]) -> T
    {
        self.data.chunks(self.stride).map(transform)
    }

    /// Indexes this structure's content.
    /// 
    /// # Arguments
    /// 
    /// * `index` - A range of elements to collect.
    /// 
    /// # Panics
    /// 
    /// Panics if `index.start * self.stride > self.data.len()` or `index.end * self.stride > self.data.len()`.
    pub fn index(&self, index: Range<usize>) -> Chunks<'a, u8> {
        let range = Range {
            start: index.start * self.stride,
            end: index.end * self.stride
        };

        self.data[range].chunks(self.stride)
    }

    /// Returns a single element in this structure.
    /// 
    /// # Arguments
    /// 
    /// * `index` - The index of the element to return.
    /// 
    /// # Panics
    /// 
    /// Panics if `index * self.stride > self.data.len()`.
    pub fn at(&self, index: usize) -> &'a [u8] {
        let range = Range {
            start: index * self.stride,
            end: (index + 1) * self.stride
        };

        &self.data[range]
    }

    pub fn iter(&self) -> Chunks<'a, u8> {
        self.data.chunks(self.stride)
    }

    pub fn len(&self) -> usize { self.data.len() / self.stride }
}