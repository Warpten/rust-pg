use smallvec::SmallVec;
use crate::dbcd::typed::map::Table;
use super::dbd::{Definition, StructureDefinition};

pub trait Parsable : Sized {
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
    I64(SmallVec<[i64; 4]>),
    U64(SmallVec<[u64; 4]>),
    String(SmallVec<[String; 4]>),
    F32(SmallVec<[f32; 4]>),
    F64(SmallVec<[f64; 4]>),
    ForeignKey(u32),
    Empty,
}
