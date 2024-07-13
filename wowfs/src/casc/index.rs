use std::{fs::File, io::{BufReader, Read}, ops::Range, path::{Path, PathBuf}};
use std::fmt::{Debug, Formatter};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use bytes::Buf;

use super::{blte::BLTE, errors::Error};

pub struct Index {
    path : PathBuf,
    bucket : u8,
    pub spec : EntrySpec,
    entry_count : u32,
    buffer : Vec<u8>,
}
impl Index {
    pub fn new<P>(path : P) -> Result<Self, Error> where P : AsRef<Path> {
        match std::fs::read(&path) {
            Ok(file) => Self::read(path, &file[..]),
            Err(_) => Err(Error::FileNotFound(path.as_ref().to_path_buf())),
        }
    }

    fn read<P>(path : P, source : &[u8]) -> Result<Self, Error> where P : AsRef<Path> {
        let mut cursor = source;

        // TODO: validate hashes
        let _hash_size = cursor.get_u32_le();
        let _hash = cursor.get_u32_le();

        let version = cursor.get_u16_le();
        assert_eq!(version, 7);

        let bucket = cursor.get_u8();
        let extra_bytes = cursor.get_u8();
        assert_eq!(extra_bytes, 0);

        let spec = EntrySpec {
            size : cursor.get_u8(),
            offset : cursor.get_u8(),
            key : cursor.get_u8(),
            offset_bits : cursor.get_u8(),
        };

        let _archive_size = cursor.get_u64_le();
        // assert_eq!(archive_size, 0x4000000000);

        // Padding is aligned to 0x10:
        let padding = ((unsafe { cursor.as_ptr().offset_from(source.as_ptr()) as usize }) + 8) & !7;
        assert_eq!(padding, 0x20);

        cursor = &source[padding..];

        let entries_size = cursor.get_u32_le();
        let _entries_hash = cursor.get_u32_le();

        let mut buffer = Vec::<u8>::with_capacity(entries_size as _);
        buffer.resize(entries_size as _, 0);
        _ = cursor.read_exact(&mut buffer[..]);

        Ok(Self {
            path : path.as_ref().to_path_buf(),
            bucket,
            buffer,
            entry_count : entries_size / (spec.key as u32 + spec.offset as u32 + spec.size as u32),
            spec,
        })
    }

    pub fn bucket(&self) -> u8 { self.bucket }
    pub fn entry_count(&self) -> u32 { self.entry_count }

    pub fn entry(&self, index : usize) -> Option<Entry> {
        let range : Range<usize> = Range {
            start : index * (self.spec.size + self.spec.key + self.spec.offset) as usize,
            end : (index + 1) * (self.spec.size + self.spec.key + self.spec.offset) as usize,
        };
        if range.end <= self.buffer.len() {
            Some(Entry(self, range))
        } else {
            None
        }
    }
}

pub struct EntrySpec {
    size : u8,
    offset : u8,
    key : u8,
    offset_bits : u8,
}
impl EntrySpec {
    pub fn key(&self) -> Range<usize> {
        Range { start : 0, end : self.key as _ }
    }

    pub fn size(&self) -> Range<usize> {
        Range {
            start : self.key as usize + self.offset as usize,
            end : self.key as usize + self.offset as usize + self.size as usize
        }
    }

    pub fn offset(&self) -> Range<usize> {
        Range {
            start : self.key as usize,
            end : self.key as usize + self.offset as usize
        }
    }
}

pub struct Entry<'a>(&'a Index, Range<usize>);
impl Entry<'_> {
    pub fn materialize(&self) -> (&[u8], u64, (u64, u64)) {
        (self.key(), self.size(), self.offset())
    }

    pub fn key(&self) -> &[u8] {
        let range = &self.1;
        let record = &self.0.buffer[range.start..range.end];

        &record[self.0.spec.key()]
    }

    pub fn size(&self) -> u64 {
        let range = &self.1;
        let record = &self.0.buffer[range.start..range.end];

        let mut record = &record[self.0.spec.size()];
        record.read_uint::<LittleEndian>(self.0.spec.size as _).unwrap()
    }

    pub fn offset(&self) -> (u64, u64) {
        let range = &self.1;
        let record = &self.0.buffer[range.start..range.end];

        let mut record = &record[self.0.spec.offset()];
        let raw_value = record.read_uint::<BigEndian>(self.0.spec.offset as _)
            .expect("Failed to read offset according to spec");

        let archive_bits = self.0.spec.offset * 8 - self.0.spec.offset_bits;
        let offset_bits = self.0.spec.offset_bits;

        (
            (raw_value >> offset_bits) & ((1 << archive_bits) - 1),
            (raw_value & ((1 << offset_bits) - 1))
        )
    }

    /// Returns the actual binary data of the file, or an error if opening failed.
    pub fn read(self) -> Result<BLTE, Error> {
        let (archive_index, archive_offset) = self.offset();
        let size = self.size();

        let path = self.0.path.parent().unwrap().join(format!("data.{:03}", archive_index));
        // Skip over the header preceding the BLTE data
        BLTE::new(path, archive_offset + 0x10 + 4 + 2 + 4 + 4, size)
    }
}

impl Debug for Entry<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (archive_index, archive_offset) = self.offset();
        let size = self.size();
        let key = self.key();

        write!(f, "({:?}, {}, {}, {})", key, archive_index, archive_offset, size)
    }
}