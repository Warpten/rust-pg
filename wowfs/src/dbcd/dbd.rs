use std::{collections::HashMap, io::{BufRead, Lines}, ops::Deref, slice::Iter};
use std::ops::Index;
use once_cell::sync::Lazy;
use regex::Regex;

/// A definition for various versions of a DBC/DB2 file.
#[derive(Debug)]
pub struct Definition {
    name: String,
    columns: Vec<ColumnDefinition>,
    structures: Vec<StructureDefinition>,
}
impl Definition {
    pub fn new<R>(lines : Lines<R>, name : String) -> Option<Self> where R : BufRead {
        let mut column_definitions = vec![];
        let mut structures = vec![];

        let mut lines = lines.map(Result::unwrap);

        let mut state = DefinitionParsingState::None;

        loop {
            if let Some(line) = lines.next() {
                if line.is_empty() || line.starts_with("COMMENT") {
                    continue;
                }

                if line.starts_with("BUILD") || line.starts_with("LAYOUT") {
                    match state {
                        DefinitionParsingState::None => return None,
                        DefinitionParsingState::ColumnDefinitions => {
                            state = DefinitionParsingState::Structure { subjects: vec![], columns: vec![] };
                        }
                        DefinitionParsingState::Structure { ref mut columns, ref mut subjects } => {
                            if !columns.is_empty() {
                                // Update column indices
                                for i in 0..columns.len() {
                                    columns[i].index = i;
                                }

                                let mut collected_columns = vec![];
                                let mut collected_subjects = vec![];
                                std::mem::swap(&mut collected_columns, columns);
                                std::mem::swap(&mut collected_subjects, subjects);

                                structures.push(StructureDefinition {
                                    columns: collected_columns,
                                    subjects: collected_subjects,
                                });
                            }
                        }
                    };
                }

                match state {
                    DefinitionParsingState::None => {
                        if line == "COLUMNS" {
                            state = DefinitionParsingState::ColumnDefinitions;
                        } else {
                            return None
                        }
                    },
                    DefinitionParsingState::ColumnDefinitions => {
                        match ColumnDefinition::try_from(line.as_str()) {
                            Ok(column) => column_definitions.push(column),
                            Err(_) => return None
                        };
                    },
                    DefinitionParsingState::Structure { ref mut subjects, ref mut columns } => {
                        if line.starts_with("LAYOUT") {
                            // Skip "LAYOUT "
                            line[7..].split(&[' ', ','])
                                .filter_map(|l| u32::from_str_radix(l, 16).ok())
                                .map(|hash| StructureSubject::LayoutHash(hash))
                                .for_each(|subject| subjects.push(subject))
                                ;
                        } else if line.starts_with("BUILD") {
                            // Skip "BUILD "
                            line[6..].split(&[',', ' '])
                                .filter_map(|token| {
                                    if let Some(offset) = token.find('-') {
                                        let from = Build::try_from(&token[0..offset]).ok();
                                        let to = Build::try_from(&token[offset + 1..]).ok();
                                    
                                        from.zip(to)
                                            .map(|(from, to)| StructureSubject::BuildRange { from, to })
                                    } else if !token.is_empty() {
                                        Build::try_from(token)
                                            .map(|v| StructureSubject::Build(v))
                                            .ok()
                                    } else {
                                        None
                                    }
                                })
                                .for_each(|subject| subjects.push(subject))
                                ;
                        } else {
                            if let Ok(column) = ColumnReference::try_from(line.as_str()) {
                                columns.push(column);
                            } else {
                                panic!("Unparsable line: {}", line)
                            }
                        }
                    },
                }
            } else {
                break;
            }
        }

        // Don't forget about the last structure
        if let DefinitionParsingState::Structure { subjects, columns } = state {
            if !subjects.is_empty() && !columns.is_empty() {
                structures.push(StructureDefinition {
                    columns,
                    subjects
                });
            }
        }

        Some(Self {
            name,
            columns: column_definitions,
            structures,
        })
    }

    #[inline] pub fn columns(&self) -> &[ColumnDefinition] { &self.columns }
    #[inline] pub fn structures(&self) -> &[StructureDefinition] { &self.structures }

    pub fn spec(&self, reference: &ColumnReference) -> Option<&ColumnDefinition> {
        self.columns.iter().find(|c| c.name == reference.name)
    }

    pub fn select(&self, layout_hash : Option<u32>, build: Option<Build>) -> Option<&StructureDefinition> {
        // Note: this barfs when multiple layouts are tagged by layout hash but also tagged by build range
        //       However this is technically specific to blizzard changing layout but not causing a layout
        //       hash recalc. If identifying the spec starts to fail this function is the culprit, because
        //       the semantics of DBD do not (according to Marla) allow duplicate layout hashes. Godspeed.

        layout_hash.and_then(|hash| {
            let subject = StructureSubject::LayoutHash(hash);

            self.structures
                .iter()
                .find(|structure| structure.is_eligible_for(&subject))
        }).or_else(|| {
            build.and_then(|build| {
                let subject = StructureSubject::Build(build);

                self.structures
                    .iter()
                   .find(|structure| structure.is_eligible_for(&subject))
            })
        })
    }
}

enum DefinitionParsingState {
    None,
    ColumnDefinitions,
    Structure {
        subjects: Vec<StructureSubject>,
        columns: Vec<ColumnReference>,
    }
}

/// Models the definition of a column according to the DBD file format.
#[derive(Debug)]
pub struct ColumnDefinition {
    pub column_type: ColumnType,
    name: String,
    fk: Option<(String, String)>
}
impl TryFrom<&str> for ColumnDefinition {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        const REGEX_EXPR : Lazy<Regex> = Lazy::new(|| {
            Regex::new("^([a-z]+)(?:<(.+)::(.+)>)? ([^ ]+).*$").unwrap()
        });

        match REGEX_EXPR.captures(value) {
            Some(captures) => {
                let column_type = match &captures[1] {
                    "int" => ColumnType::Integer,
                    "float" => ColumnType::Float,
                    "string" => ColumnType::String,
                    "locstring" => ColumnType::LocalizedString,
                    _ => return Err(())
                };

                let name = {
                    let name = &captures[4];
                    let length = match name.ends_with('?') {
                        true => name.len() - 1,
                        false => name.len()
                    };

                    &name[0..length]
                }.to_string();

                let fk = captures.get(2).zip(captures.get(3))
                    .map(|(t, c)| (t.as_str().to_owned(), c.as_str().to_owned()));

                Ok(Self {
                    column_type,
                    name,
                    fk
                })
            },
            None => Err(())
        }
    }
}

/// Models the definition of a structure according to the DBD file format.
/// 
/// A structure is used to parse a DBC or DB2 file. It defines a number of [subjects](StructureSubject) that determines
/// builds and/or layout hashes for which the structure definition is applicable.
#[derive(Debug)]
pub struct StructureDefinition {
    columns: Vec<ColumnReference>,
    subjects: Vec<StructureSubject>,
}
impl StructureDefinition {
    pub fn is_eligible_for(&self, subject : &StructureSubject) -> bool {
        self.subjects.iter().any(|itr| itr.contains(subject))
    }

    pub fn iter(&self) -> Iter<'_, ColumnReference> {
        self.columns.iter()
    }

    pub fn get(&self, index: usize) -> Option<&ColumnReference> {
        self.columns.get(index)
    }

    pub fn name(&self, name: &str) -> Option<&ColumnReference> {
        self.columns.iter().find(|c| c.name == name)
    }
}
impl Index<usize> for StructureDefinition {
    type Output = ColumnReference;

    fn index(&self, index: usize) -> &Self::Output {
        &self.columns[index]
    }
}
impl Index<&str> for StructureDefinition {
    type Output = ColumnReference;

    fn index(&self, index: &str) -> &Self::Output {
        self.name(index).unwrap()
    }
}

impl Deref for StructureDefinition {
    type Target = [ColumnReference];

    fn deref(&self) -> &Self::Target {
        &self.columns[..]
    }
}

/// This object references a [`ColumnDefinition`] but adds specifics on top that are relative to the encapsulating
/// [`StructureDefinition`].
#[derive(Debug)]
pub struct ColumnReference {
    pub name: String,
    pub arity: u32,
    pub bits: Option<u32>,

    // These are computed late
    pub index: usize,

    pub unsigned: bool,
    pub id: bool,
    pub noninline: bool,
    pub relation: bool,
}
impl ColumnReference {
}

impl TryFrom<&str> for ColumnReference {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        static COLUMN_EXPR: Lazy<Regex> = Lazy::new(|| unsafe {
            Regex::new(r"(\$(?P<attrs>,?[^$]+)+\$)?(?P<name>[^<\[ ]+)(?:<(?P<unsigned>u)?(?P<bits>[0-9]+)>)?(?:\[(?P<arity>[0-9]+)\])?").unwrap_unchecked()
        });

        if let Some(captures) = COLUMN_EXPR.captures(value) {
            let attributes : Vec<_> = captures.name("attrs").map(|capture| {
                capture.as_str()
            }).unwrap_or("").split(',').collect();
            let name = captures["name"].to_string();

            let bits = captures.name("bits").and_then(|capture| {
                u32::from_str_radix(capture.as_str(), 10).ok()
            });
            let arity = captures.name("arity").and_then(|capture| {
                u32::from_str_radix(capture.as_str(), 10).ok()
            }).unwrap_or(1);

            let id = attributes.contains(&"id");
            let noninline = attributes.contains(&"noninline");
            let relation = attributes.contains(&"relation");
            let unsigned = captures.name("unsigned").is_some();

            Ok(Self {
                arity,
                name,
                bits,

                index: 0,

                unsigned,
                id,
                noninline,
                relation,
            })
        } else {
            Err(())
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum StructureSubject {
    BuildRange { from: Build, to: Build },
    Build(Build),
    LayoutHash(u32)
}
impl StructureSubject {
    pub fn contains(&self, other: &StructureSubject) -> bool {
        match (self, other) {
            (Self::BuildRange { from, to }, Self::BuildRange { from: oth_from, to: oth_to }) => {
                from <= oth_from && to >= oth_to
            },
            (Self::BuildRange { from, to }, Self::Build(oth)) => {
                from <= oth && oth <= to
            },
            (Self::Build(ths), Self::Build(oth)) => ths == oth,
            (Self::LayoutHash(ths), Self::LayoutHash(oth)) => ths == oth,
            (_, _) => false,
        }
    }
}

#[derive(Eq, Ord, Copy, Clone, Debug)]
pub struct Build {
    major: u32,
    minor: u32,
    patch: u32,
    build: u32,
}
impl PartialOrd for Build {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.build.partial_cmp(&other.build)
    }
}
impl PartialEq for Build {
    fn eq(&self, other: &Self) -> bool {
        self.build == other.build
    }
}

impl From<u32> for Build {
    fn from(value: u32) -> Self {
        Self {
            major: 0,
            minor: 0,
            patch: 0,
            build: value,
        }
    }
}
impl From<&str> for Build {
    fn from(value: &str) -> Self {
        let parts : Vec<_> = value.split('.').map(|s| {
            u32::from_str_radix(s, 10).unwrap()
        }).collect();

        Self {
            major: parts[0],
            minor: parts[1],
            patch: parts[2],
            build: parts[3]
        }
    }
}
impl ToString for Build {
    fn to_string(&self) -> String {
        format!("{}.{}.{}.{}", self.major, self.minor, self.patch, self.build)
    }
}

#[derive(Debug)]
pub enum ColumnType {
    String,
    LocalizedString,
    Integer,
    Float,
}
impl ToString for ColumnType {
    fn to_string(&self) -> String {
        self.as_ref().into()
    }
}
impl AsRef<str> for ColumnType {
    fn as_ref(&self) -> &str {
        match self {
            ColumnType::String => "string",
            ColumnType::LocalizedString => "locstring",
            ColumnType::Integer => "int",
            ColumnType::Float => "float",
        }
    }
}

#[cfg(test)]
pub mod test {
    use std::io::BufRead;

    use super::{ColumnReference, Definition};

    #[test]
    pub fn test_column_reference() {
        let expr = "foo<u32>[3]";
        let column_reference = ColumnReference::try_from(expr).unwrap();

        assert_eq!(column_reference.unsigned, true);
        assert_eq!(column_reference.name, "foo");
        assert_eq!(column_reference.arity, 3);

        let expr = "foo[3]";
        let column_reference = ColumnReference::try_from(expr).unwrap();

        assert_eq!(column_reference.unsigned, false);
        assert_eq!(column_reference.name, "foo");
        assert_eq!(column_reference.arity, 3);
    }


    #[test]
    pub fn test_definition() {
        let data = include_bytes!("../../tests/map.dbd.sample");
        let definition = Definition::new(data.lines(), "Map.dbd".to_string()).unwrap();

        let structure = definition.select(Option::Some(0xEE526FA5), None).unwrap();

        assert_eq!(structure.len(), 23);

        assert_eq!(1, structure.iter().enumerate().fold(0, |acc, (_, column)| {
            if column.id { acc + 1 } else { acc }
        }));

        assert_eq!(structure[0].name, "ID");

        assert_eq!(structure["MinimapIconScale"].bits, None);

        assert_eq!(structure["CosmeticParentMapID"].unsigned, false);
        assert_eq!(structure["CosmeticParentMapID"].bits, Some(16));

        assert_eq!(structure["MapType"].unsigned, true);
        assert_eq!(structure["MapType"].bits, Some(8));
    }
}