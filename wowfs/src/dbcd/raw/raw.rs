use std::ops::{Index, Range, RangeFrom, RangeFull, RangeTo};

pub struct Raw<'a> {
    data: &'a [u8]
}
impl<'a> Raw<'a> {
    /// Constructs a new instance of this type and returns the remainder of the input data.
    /// 
    /// # Arguments
    /// 
    /// * `input` - An input slice of bytes.
    /// * `length` - The amount of bytes in this chunk.
    /// 
    /// # Panics
    ///
    /// Panics if `length > input.len()`.
    pub fn from(input: &'a [u8], length: usize) -> (Self, &'a [u8]) {
        let (data, remainder) = input.split_at(length);
        (Raw { data }, remainder)
    }

    /// Optionally constructs a new instance of this type if the given flag is `true`.
    /// 
    /// # Arguments
    /// 
    /// * `flag` - A flag indicating if data should be read.
    /// * `input` - An input slice of bytes.
    /// * `length` - The amount of bytes in this chunk.
    /// 
    /// # Returns
    /// 
    /// Returns a new instance that may be empty.
    /// 
    /// # Panics
    ///
    /// Panics if `length > input.len()`.
    #[inline]
    pub fn optionally_from(flag: bool, input: &'a [u8], length: usize) -> (Self, &'a [u8]) {
        static EMPTY : &'static[u8] = &[];

        if flag {
            Self::from(input, length)
        } else {
            (Self { data: EMPTY }, input)
        }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}
impl Index<Range<usize>> for Raw<'_> {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.data[index]
    }
}
impl Index<RangeFull> for Raw<'_> {
    type Output = [u8];

    fn index(&self, index: RangeFull) -> &Self::Output {
        &self.data[index]
    }
}
impl Index<RangeFrom<usize>> for Raw<'_> {
    type Output = [u8];

    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        &self.data[index]
    }
}
impl Index<RangeTo<usize>> for Raw<'_> {
    type Output = [u8];

    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        &self.data[index]
    }
}
impl Index<usize> for Raw<'_> {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}