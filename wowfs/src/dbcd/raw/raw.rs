pub trait RawTrait<'a> : Sized {
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
    fn from_raw(input: &'a [u8], length: usize) -> (Self, &'a [u8]);
}

pub mod details {
    use std::ops::{Deref, Index};
    use bytemuck::{cast_slice, AnyBitPattern};

    use super::RawTrait;

    pub struct Raw<T> {
        data: T
    }

    impl<'a> RawTrait<'a> for Raw<Vec<u8>> {
        fn from_raw(input: &'a [u8], length: usize) -> (Self, &'a [u8]) {
            let (data, remainder) = input.split_at(length);
            (Self { data: data.to_vec() }, remainder)
        }
    }

    impl<'a> RawTrait<'a> for Raw<&'a [u8]> {
        fn from_raw(input: &'a [u8], length: usize) -> (Self, &'a [u8]) {
            let (data, remainder) = input.split_at(length);
            (Self { data }, remainder)
        }
    }

    impl<'a, T> Raw<T> where T : Deref<Target = [u8]> + Default, Raw<T> : RawTrait<'a> {
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
            if flag {
                RawTrait::from_raw(input, length)
            } else {
                (Self { data: Default::default() }, input)
            }
        }
    }

    impl<T> Raw<T> where T : Deref<Target = [u8]> {
        pub fn data(&self) -> &[u8] {
            &self.data
        }

        pub fn cast<U>(&self) -> &[U] where U : AnyBitPattern {
            let data = self.data();
            if data.is_empty() {
                &[]
            } else {
                cast_slice(data)
            }
        }
    }

    impl<T, I> Index<I> for Raw<T> where T: Index<I> {
        type Output = <T as Index<I>>::Output;

        fn index(&self, index: I) -> &Self::Output {
            &self.data[index]
        }
    }

    impl<T> Deref for Raw<T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.data
        }
    }
}

pub type Raw<'a> = details::Raw<&'a [u8]>;
pub type RawBuf = details::Raw<Vec<u8>>;