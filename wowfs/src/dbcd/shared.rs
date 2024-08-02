use smallvec::SmallVec;

use super::dbd::{Definition, StructureDefinition};

pub trait Parser {
    /// Returns all values and all columns on all rows.
    /// 
    /// # Arguments
    /// 
    /// * `definition` - The definition to use.
    /// * `structure` - The layout chosen for parsing.
    fn parse(&self, definition: &Definition, structure: &StructureDefinition) -> Vec<Vec<RawValue>>;
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
}
