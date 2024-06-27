use std::{io::Read, ops::Range};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use bytes::Buf;

pub struct Index {
    bucket : u8,
    spec : EntrySpec,
    entry_count : u32,
    buffer : Vec<u8>,
}
impl Index {
    pub fn read(source : &[u8]) -> Result<Self, Error> {
        let mut cursor = source;

        // TODO: validate hashes
        let hash_size = cursor.get_u32_le();
        let hash = cursor.get_u32_le();

        let version = cursor.get_u32_le();
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

        let archive_size = cursor.get_u64_le();
        assert_eq!(archive_size, 0x4000000000);

        // Padding is aligned to 0x10:
        let padding = unsafe { cursor.as_ptr().offset_from(source.as_ptr()) as usize };
        let padding = round_up(padding, 0x10);
        assert_eq!(padding, 0x18);

        cursor = &source[padding..];

        let entries_size = cursor.get_u32_le();
        let entries_hash = cursor.get_u32_le();

        let mut buffer = Vec::<u8>::with_capacity(entries_size as _);
        buffer.resize(entries_size as _, 0);
        _ = cursor.read_exact(&mut buffer[..]);

        Ok(Self {
            bucket,
            buffer,
            entry_count : entries_size / (spec.key as u32 + spec.offset as u32 + spec.size as u32),
            spec,
        })
    }

    pub fn bucket(&self) -> u8 { self.bucket }
    pub fn entry_count(&self) -> u32 { self.entry_count }

    pub fn entry(&self, index : u32) -> Entry {
        let range : Range<usize> = Range {
            start : index as usize * (self.spec.size + self.spec.key + self.spec.offset) as usize,
            end : (index + 1) as usize * (self.spec.size + self.spec.key + self.spec.offset) as usize,
        };
        Entry(self, range)
    }
}

const fn round_up(base : usize, power : usize) -> usize {
    (base + power - 1) & (0 - power)
}

struct EntrySpec {
    size : u8,
    offset : u8,
    key : u8,
    offset_bits : u8,
}
pub enum Error {
    ReadError,
}

pub struct Entry<'a>(&'a Index, Range<usize>);
impl Entry<'_> {
    pub fn key(&self) -> &[u8] {
        let range = &self.1;
        let record = &self.0.buffer[range.start..range.end];

        let range : Range<usize> = Range {
            start : 0,
            end : self.0.spec.key as _
        };
        &record[range]
    }

    pub fn size(&self) -> u64 {
        let range = &self.1;
        let record = &self.0.buffer[range.start..range.end];

        let range : Range<usize> = Range {
            start : self.0.spec.key as usize + self.0.spec.offset as usize,
            end : self.0.spec.key as usize + self.0.spec.offset as usize + self.0.spec.size as usize
        };

        let mut record = &record[range];
        record.read_uint::<LittleEndian>(self.0.spec.size as _).unwrap()
    }

    pub fn offset(&self) -> (u64, u64) {
        let range = &self.1;
        let record = &self.0.buffer[range.start..range.end];

        let range : Range<usize> = Range {
            start : self.0.spec.key as usize,
            end : self.0.spec.key as usize + self.0.spec.offset as usize
        };

        let mut record = &record[range];
        let raw_value = record.read_uint::<BigEndian>(self.0.spec.offset as _).unwrap();

        let archive_bits = self.0.spec.offset * 8 - self.0.spec.offset_bits;
        let offset_bits = self.0.spec.offset_bits;

        (
            (raw_value >> offset_bits) & ((1 << archive_bits) - 1),
            (raw_value & ((1 << offset_bits) - 1))
        )
    }
}