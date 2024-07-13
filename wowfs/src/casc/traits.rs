use std::io::{BufRead, Read};


macro_rules! read {
    ($fn:ident, $type:ty) => {
        fn $fn<R, F>(source : &mut R, transform : F) -> $type where R : std::io::Read, F : FnOnce([u8; std::mem::size_of::<$type>()]) -> $type {
            let mut bytes = [0_u8; std::mem::size_of::<$type>()];
            if let Ok(_) = source.read_exact(&mut bytes) {
                transform(bytes)
            } else {
                panic!("Unable to read {} bytes from source", std::mem::size_of::<$type>())
            }
        }
    };
}

#[macro_export]
macro_rules! fixed_size {
    ($type:ty, $size:stmt, $err:expr) => {
        impl $crate::tact::traits::FixedSize for $type {
            fn size() -> usize { $size }
        
            fn read_error() -> Self::ErrorType { $err }
        }
    }
}

pub trait FixedSize : Deserializable {
    /// Returns the size of this structure, in bytes, when serialized
    fn size() -> usize;

    fn read_error() -> Self::ErrorType;

    fn deserialize<R>(source : &mut R) -> Result<Self, Self::ErrorType> where R : Read {
        let mut buffer = Vec::<u8>::with_capacity(Self::size());
        buffer.resize(Self::size(), 0);
        if let Ok(_) = source.read_exact(&mut buffer[..]) {
            Self::read(&buffer[..])
        } else {
            Err(Self::read_error())
        }
    }
}

pub trait Deserializable : Sized {
    type ErrorType;

    fn read(source : &[u8]) -> Result<Self, Self::ErrorType>;

    fn read_u8<R>(source : &mut R) -> u8 where R : std::io::Read {
        let mut bytes = [0_u8; 1];
        if let Ok(_) = source.read_exact(&mut bytes) {
            bytes[0]
        } else {
            panic!("Unable to read {} bytes from source", 1)
        }
    }
    
    fn read_i8<R>(source : &mut R) -> i8 where R : std::io::Read {
        let mut bytes = [0_u8; 1];
        if let Ok(_) = source.read_exact(&mut bytes) {
            bytes[0] as _
        } else {
            panic!("Unable to read {} bytes from source", 1)
        }
    }

    read! { read_u128, u128 }
    read! { read_u64, u64 }
    read! { read_u32, u32 }
    read! { read_u16, u16 }

    read! { read_i128, i128 }
    read! { read_i64, i64 }
    read! { read_i32, i32 }
    read! { read_i16, i16 }

    read! { read_f32, f32 }
}
