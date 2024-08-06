use std::ops::Range;

use bytemuck::cast_slice;
use bytes::Buf;
use smallvec::SmallVec;

use crate::dbcd::{dbd::{ColumnDefinition, ColumnReference, ColumnType, Definition}, structured::{ExtendedFieldInfo, FieldCompressionType}, typed::table::Table};

pub trait DBC : Sized {
    /// Parses this DBC file into a [`Table`]. If parsing fails, returns an empty result.
    ///
    /// # Arguments
    ///
    /// * `definition` - The definition to use for parsing.
    fn parse_table(self, definition: &Definition) -> Option<Table>;

    /// Parses this DBC file and returns a vector of column values. If parsing fails, returns an empty
    /// vector.
    ///
    /// # Arguments
    ///
    /// * `definition` - The definition to use for parsing.
    fn parse(self, definition: &Definition) -> Vec<Vec<RawValue>>;
}

#[derive(Debug)]
pub enum RawValue {
    I8(SmallVec<[i8; 4]>),
    U8(SmallVec<[u8; 4]>),
    I16(SmallVec<[i16; 4]>),
    U16(SmallVec<[u16; 4]>),
    I32(SmallVec<[i32; 4]>),
    U32(SmallVec<[u32; 4]>),
    I64(SmallVec<[i64; 2]>),
    U64(SmallVec<[u64; 2]>),
    String(SmallVec<[String; 2]>),
    F32(SmallVec<[f32; 4]>),
    F64(SmallVec<[f64; 2]>),
    ForeignKey(u32),
    Empty,
}

fn read_bitpacked(values: &[u8], field_info: &ExtendedFieldInfo) -> Vec<u64> {
    // At most, 64 bits, aka 8 bytes
    let byte_width = field_info.field.size_bytes();
    let shift_offset = 64 - field_info.field.size_bits() - (field_info.field.offset_bits() % 8);
    let mask_offset = 64 - field_info.field.size_bits();

    let mut cursor = values;

    let mut result = Vec::with_capacity(values.len());

    for _ in 0..(values.len() / byte_width) {
        let raw_integer = cursor.get_uint_le(byte_width);
        let value = (raw_integer << shift_offset) >> mask_offset;
        result.push(value);
    }

    result
}
