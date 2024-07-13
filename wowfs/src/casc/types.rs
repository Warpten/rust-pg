use std::{cmp::Ordering, fmt::{Debug, Display, Formatter}, ops::Index, slice::SliceIndex};

macro_rules! make_key {
    ($t:ident) => {
        #[derive(Debug, Eq, Hash, PartialEq)]
        pub struct $t(Vec<u8>);
        impl $t {
            pub fn len(&self) -> usize { self.0.len() }

            pub fn new<R : std::io::Read>(source : &mut R, size : usize) -> Self {
                let mut buffer = vec![0; size];
                source.read_exact(&mut buffer).unwrap();
                Self::from(&buffer[..])
            }
        }
        impl From<&str> for $t {
            fn from(value: &str) -> Self {
                Self((0..value.len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
                        .collect::<Vec<_>>()
                )
            }
        }
        impl From<&[u8]> for $t {
            fn from(value: &[u8]) -> Self {
                Self(Vec::from(value))
            }
        }
        impl<I : SliceIndex<[u8]>> Index<I> for $t {
            type Output = I::Output;
        
            fn index(&self, index: I) -> &Self::Output {
                &self.0[index]
            }
        }
        impl Display for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
        impl PartialEq<[u8]> for $t {
            fn eq(&self, other: &[u8]) -> bool {
                let len = std::cmp::min(self.0.len(), other.len());
                self.0[0..len] == other[0..len]
            }
        }
        impl PartialOrd<[u8]> for $t {
            fn partial_cmp(&self, other: &[u8]) -> Option<std::cmp::Ordering> {
                if self.0.len() == other.len() {
                    for i in 0..other.len() {
                        match self.0[i].cmp(&other[i]) {
                            std::cmp::Ordering::Equal => (),
                            std::cmp::Ordering::Less => return Some(std::cmp::Ordering::Less),
                            std::cmp::Ordering::Greater => return Some(std::cmp::Ordering::Greater),
                        };
                    }
        
                    Some(Ordering::Equal)
                } else {
                    None
                }
            }
        }
    };
}

make_key! { ContentKey }
make_key! { EncodingKey }
make_key! { ArchiveKey }

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(crate) struct FileDataID(pub(crate) u32);

impl Display for FileDataID {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
