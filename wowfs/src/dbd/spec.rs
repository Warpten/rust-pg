use std::io::{BufRead, BufReader, Lines, Read};
use bytes::Buf;

pub struct Definition {
    columns: Vec<ColumnDefinition>,
    structures: Vec<Structure>,
}
impl Definition {
    pub fn read<R : BufRead>(lines: &mut Lines<R>) -> Option<Definition> {
        let columns = details::read_columns(lines);
        let mut structures : Vec<Structure> = vec![];

        loop {
            if let Some(build) = details::read_build(lines, &columns) {
                structures.push(build);
            } else {
                break;
            }
        }

        if columns.is_empty() || structures.is_empty() {
            None
        } else {
            Some(Definition {
                columns,
                structures,
            })
        }
    }
}

mod details {
    use std::io::{BufRead, Lines};
    use std::ops::Range;
    use crate::dbd::spec::{BuildSpec, Column, ColumnDefinition, ColumnType, Structure};

    pub fn read_build<B : BufRead>(source: &mut Lines<B>, column_defs: &[ColumnDefinition]) -> Option<Structure> {
        let mut builds : Vec<BuildSpec> = vec![];
        let mut hashes : Vec<u32> = vec![];
        let mut columns : Vec<Column> = vec![];

        let mut column_index = 0;
        loop {
            if let Some(line) = source.next().map(|l| l.unwrap()) {
                if line.len() == 0 { // Done with build section
                    break;
                }

                if line.starts_with("LAYOUT ") {
                    line[7..].split(&[' ', ','])
                        .filter(|s| !s.is_empty())
                        .map(|s| u32::from_str_radix(s, 16).unwrap())
                        .for_each(|hash| hashes.push(hash));
                }
                else if line.starts_with("BUILD ") {
                    line[6..].split(&[',', ' '])
                        .filter(|s| !s.is_empty())
                        .map(|build| {
                            match build.split_once('-') {
                                Some((l, r)) => BuildSpec::Range { from: l.into(), to: r.into() },
                                None => BuildSpec::Single(build.into())
                            }
                        })
                        .for_each(|build| builds.push(build));
                } else {
                    let mut tokens : Vec<_> = line.split(&['$', ',', '<', '>']).collect();
                    assert!(!tokens.is_empty());

                    let bits = u32::from_str_radix(tokens[tokens.len() - 1], 10).ok();
                    let (column, attributes) = if bits.is_some() {
                        (tokens[tokens.len() - 2], &tokens[Range { start: 0, end: tokens.len() - 2 }])
                    } else {
                        (tokens[tokens.len() - 1], &tokens[Range { start: 0, end: tokens.len() - 1 }])
                    };
                    let definition_index = column_defs
                        .iter()
                        .enumerate()
                        .find_map(|(i, c)| {
                            if c.name == column {
                                Some(i)
                            } else {
                                None
                            }
                        })
                        .unwrap();

                    let noninline = attributes.contains(&"noninline");
                    let id = attributes.contains(&"id");
                    let relation = attributes.contains(&"relation");

                    columns.push(Column {
                        index: column_index,
                        definition_index,
                        bits,

                        noninline,
                        id,
                        relation,
                    });

                    column_index += 1;
                }
            } else {
                break;
            }
        }

        None
    }

    pub fn read_columns<B : BufRead>(source : &mut Lines<B>) -> Vec<ColumnDefinition> {
        let mut columns : Vec<ColumnDefinition> = vec![];

        loop {
            if let Some(line) = source.next().map(|l| l.unwrap()) {
                if line.len() == 0 { // Done with columns section
                    break;
                }

                if line.starts_with("COLUMNS") {
                    continue;
                }

                let tokens : Vec<_> = line.split(&['<', '>', ' ', ':']).collect();

                let column_type = match tokens[0] {
                    "int" => ColumnType::Integer,
                    "float" => ColumnType::Float,
                    "string" => ColumnType::String,
                    "locstring" => ColumnType::LocalizedString,
                    _ => panic!("Unsupported column type {}", tokens[0])
                };

                let name = tokens.last()
                    .unwrap()
                    .strip_suffix('?')
                    .unwrap_or(tokens.last().unwrap())
                    .to_owned();

                let fk = if tokens.len() == 2 {
                    None
                } else if tokens.len() == 4 {
                    Some((tokens[1].to_owned(), tokens[2].to_owned()))
                } else {
                    panic!("Fuck!")
                };

                columns.push(ColumnDefinition {
                    column_type,
                    name,
                    fk
                });
            } else {
                break;
            }
        }
        columns
    }
}

pub struct Structure {
    columns: Vec<Column>,
    builds: Vec<BuildSpec>,
    layouts: Vec<u32>,
}

pub struct ColumnDefinition {
    column_type: ColumnType,
    name: String,
    fk: Option<(String, String)>,
}
pub enum ColumnType {
    Integer,
    Float,
    String,
    LocalizedString,
}

pub struct Column {
    index: usize,
    definition_index: usize,
    bits: Option<u32>,

    id: bool,
    noninline: bool,
    relation: bool,
}

pub enum BuildSpec {
    Single(Build),
    Range { from: Build, to: Build }
}

pub struct Build(u32);
impl From<&str> for Build {
    fn from(value: &str) -> Self {
        Self(value.chars().filter(|c| *c >= '0' && *c <= '9')
            .fold(0, |i, c| {
                // SAFETY: characters are 0-9
                i * 10 + unsafe { c.to_digit(10).unwrap_unchecked() }
            }))
    }
}
