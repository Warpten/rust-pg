use std::fmt::{Debug, Display, Formatter};

use bytes::Buf;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ArchiveKey(pub(crate) u128);

impl Display for ArchiveKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ContentKey {
    buffer : Vec<u8>
}

impl ContentKey {
    pub fn from(buf : &mut dyn Buf, size : usize, advance : bool) -> ContentKey {
        let key = Self::new(&buf.chunk()[0..size]);
        if advance {
            buf.advance(size);
        }
        key
    }

    pub fn new(buf: &[u8]) -> ContentKey {
        ContentKey { buffer : buf.to_vec() }
    }

    pub fn len(&self) -> usize { self.buffer.len() }
}

impl Display for ContentKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.buffer.fmt(f)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct EncodingKey {
    buffer : Vec<u8>
}

impl EncodingKey {
    pub fn from(buf : &mut dyn Buf, size : usize, advance : bool) -> EncodingKey {
        let key = Self::new(&buf.chunk()[0..size]);
        if advance {
            buf.advance(size);
        }
        key
    }

    pub fn new(buf: &[u8]) -> EncodingKey {
        EncodingKey { buffer : buf.to_vec() }
    }

    pub fn len(&self) -> usize { self.buffer.len() }
}

impl Display for EncodingKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.buffer.fmt(f)
    }
}


#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(crate) struct FileDataID(pub(crate) u32);

impl Display for FileDataID {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
