use std::{iter::Map, ops::Range, slice::Chunks};

pub trait ChunkedTrait<'a> : Sized {
    /// Constructs a new instance of this type and returns the remainder of the input data.
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
    fn from_raw(input: &'a [u8], stride: usize, count: usize) -> (Self, &'a [u8]);
}

pub mod details {
    use std::iter::Map;
    use std::ops::{Deref, Range};
    use std::slice::Chunks;
    use crate::dbcd::raw::ChunkedTrait;

    pub struct Chunked<T> {
        data: T,
        stride: usize,
    }

    impl<'a> ChunkedTrait<'a> for Chunked<Vec<u8>> {
        fn from_raw(input: &'a [u8], stride: usize, count: usize) -> (Self, &'a [u8]) {
            let (data, remainder) = input.split_at(stride * count);
            (Self { data: data.to_vec(), stride }, remainder)
        }
    }

    impl<'a> ChunkedTrait<'a> for Chunked<&'a [u8]> {
        fn from_raw(input: &'a [u8], stride: usize, count: usize) -> (Self, &'a [u8]) {
            let (data, remainder) = input.split_at(stride * count);
            (Self { data, stride }, remainder)
        }
    }

    impl<'a, T> Chunked<T> where T : Deref<Target = [u8]> + Default, Chunked<T> : ChunkedTrait<'a> {
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
            if flag {
                ChunkedTrait::from_raw(input, stride, count)
            } else {
                (Self { data: Default::default(), stride }, input)
            }
        }

        pub fn stride(&self) -> usize { self.stride }
    }

    impl<T> Chunked<T> where T : Deref<Target = [u8]> {
        /// Parses each of the subelements of this chunk's stride through the provided transformation
        /// operation.
        ///
        /// # Arguments
        ///
        /// * `transform` - A lambda that transforms a slice of bytes.
        pub fn materialize<U, F>(&self, transform : F) -> Map<Chunks<'_, u8>, F>
            where F : FnMut(&[u8]) -> U
        {
            self.data.chunks(self.stride).map(transform)
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
        pub fn at(&self, index: usize) -> &[u8] {
            let range = Range {
                start: index * self.stride,
                end: (index + 1) * self.stride
            };

            &self.data[range]
        }

        pub fn iter(&self) -> Chunks<'_, u8> {
            self.data.chunks(self.stride)
        }

        pub fn len(&self) -> usize { self.data.len() / self.stride }
    }

    impl Chunked<&[u8]> {
        pub fn to_owned(&self) -> Chunked<Vec<u8>> {
            Chunked { data: self.data.to_vec(), stride: self.stride }
        }
    }
}

/// A chunked chunk effectively slices its content in subelements with the given `stride`.
///
/// This is the owning counterpart to [`Chunked`].
pub type ChunkedBuf = details::Chunked<Vec<u8>>;

/// A chunked chunk effectively slices its content in subelements with the given `stride`.
pub type Chunked<'a> = details::Chunked<&'a [u8]>;