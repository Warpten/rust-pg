use smallvec::SmallVec;
use crate::dbcd::typed::map::Table;
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

pub trait Parsable {
    /// Parses this DBC file with the given definition and structure. If parsing fails, returns an empty vector.
    ///
    /// # Arguments
    ///
    /// * `definition` - The definition of that file according to DBD.
    /// * `spec` - A structure definition from DBD that was deemed usable for this file.
    fn parse_with(self, definition: &Definition, spec: &StructureDefinition) -> Vec<Vec<RawValue>>;

    /// Selects an appropriate DBD structure definition from the given definition for this file.
    ///
    /// # Arguments
    ///
    /// * `definition` - The definition from which to select a structure definition.
    fn select_structure(&self, definition: &Definition) -> Option<&StructureDefinition>;

    /// Parses this DBC file into a [`Table`]. If parsing fails, returns an empty result.
    ///
    /// # Arguments
    ///
    /// * `definition` - The definition to use for parsing.
    fn parse_table(self, definition: &Definition) -> Option<Table> {
        if let Some(spec) = self.select_structure(definition) {
            let records = self.parse_with(definition, spec);

            Some(Table::new(spec, records))
        } else {
            None
        }
    }

    /// Parses this DBC file and returns a vector of column values. If parsing fails, returns an empty
    /// vector.
    ///
    /// # Arguments
    ///
    /// * `definition` - The definition to use for parsing.
    fn parse(self, definition: &Definition) -> Vec<Vec<RawValue>> {
        if let Some(spec) = self.select_structure(definition) {
            self.parse_with(definition, spec)
        } else {
            vec![]
        }
    }
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
