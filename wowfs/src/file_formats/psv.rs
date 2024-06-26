use std::{fs::File, io::{BufRead, BufReader, Read}, ops::Deref, path::Path};

/// A PSV file parser implementation.
/// 
/// A PSV file is made of multiple lines where values are separated by the pipe character (|), hence their name: "pipe-delimited values",
/// a direct reference to the CSV file format. Column names are included in the file; they are separated by the same character but also
/// include more information related to how the values should be treated.
/// 
/// The column specification is formatted like `<name>!<type>:<width>`:
/// * `name` - The name of the column.
/// * `type` - The type of the value. This can be any string, but it usually is one of `DEC`, `HEX`, or `STRING`.
/// * `width` - The width of the value. This value is interpreted differently depending on the type of the value.
///   * For `DEC` values, it represents the amount of bytes used to represent the value.
///   * For `STRING` values, this value should always be zero and can be ignored.
///   * For `HEX` values, this value is the amount of bytes used to store the hex string values.
pub struct PSV {
    columns : Vec<(String, Type)>,
    record_size : usize,
    values : Vec<Value>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Type {
    String,
    Dec(usize),
    Hex(usize)
}

pub struct Value(String, Type);

impl Value {
    pub fn raw(&self) -> &str { &self.0 }

    pub fn string(&self) -> Result<&str, Error> {
        if self.1 != Type::String {
            Err(Error::InvalidColumnType)
        } else {
            Ok(&self.0)
        }
    }

    pub fn strings(&self) -> Result<Vec<&str>, Error> {
        if self.1 != Type::String {
            Err(Error::InvalidColumnType)
        } else {
            Ok(self.0.split(' ').collect())
        }
    }

    pub fn dec(&self) -> Result<u32, Error> {
        if let Type::Dec(width) = self.1 {
            let value = self.0.parse::<u32>().unwrap();
            if value < (1 << (width * 8)) {
                Ok(value)
            } else {
                Err(Error::OutOfBounds(width))
            }
        } else {
            Err(Error::InvalidColumnType)
        }
    }

    pub fn hex(&self, reverse : bool) -> Result<Vec<u8>, Error> {
        if let Type::Hex(width) = self.1 {
            if self.0.len() & 1 != 0 {
                return Err(Error::DecodingError);
            }

            let mut buffer = Vec::<u8>::with_capacity(width);

            if reverse {
                for index in (0..self.0.len()).step_by(2) {
                    let index = self.0.len() - index - 1;
                    match u8::from_str_radix(&self.0[index..index + 2], 16) {
                        Ok(c) => buffer.push(c),
                        Err(_) => return Err(Error::OutOfBounds(0xFF)),
                    }
                }
            } else {
                for index in (0..self.0.len()).step_by(2) {
                    match u8::from_str_radix(&self.0[index..index + 2], 16) {
                        Ok(c) => buffer.push(c),
                        Err(_) => return Err(Error::OutOfBounds(0xFF)),
                    }
                }
            }

            Ok(buffer)
        } else {
            Err(Error::InvalidColumnType)
        }
    }

    pub fn bool(&self) -> Result<bool, Error> {
        return self.dec().map(|v| v == 1)
    }
}

pub struct OptionalValue<'a>(Option<&'a Value>);
impl<'a> OptionalValue<'a> {
    pub fn try_raw(self : OptionalValue<'a>) -> Result<&'a str, Error> {
        if let Some(this) = self.0 {
            Ok(this.raw())
        } else {
            Err(Error::UnknownColumnName)
        }
    }

    pub fn try_string(self : OptionalValue<'a>) -> Result<&'a str, Error> {
        if let Some(this) = self.0 {
            this.string()
        } else {
            Err(Error::UnknownColumnName)
        }
    }

    pub fn try_strings(self : OptionalValue<'a>) -> Result<Vec<&'a str>, Error> {
        if let Some(this) = self.0 {
            this.strings()
        } else {
            Err(Error::UnknownColumnName)
        }
    }
    
    pub fn try_dec(self) -> Result<u32, Error> {
        if let Some(this) = self.0 {
            this.dec()
        } else {
            Err(Error::UnknownColumnName)
        }
    }

    pub fn try_bool(self) -> Result<bool, Error> {
        if let Some(this) = self.0 {
            this.bool()
        } else {
            Err(Error::UnknownColumnName)
        }
    }

    pub fn try_hex(self, reverse : bool) -> Result<Vec<u8>, Error> {
        if let Some(this) = self.0 {
            this.hex(reverse)
        } else {
            Err(Error::UnknownColumnName)
        }
    }
}
impl<'a> Deref for OptionalValue<'a> {
    type Target = Option<&'a Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Record<'a>(&'a PSV, usize);

impl Record<'_> {
    pub fn index(&self) -> usize { self.1 }

    pub fn read(&self, column : &'static str) -> OptionalValue {
        let column_index = self.0.columns.iter().position(|(column_name, _)| {
            *column_name == column
        });

        if let Some(column_index) = column_index {
            OptionalValue(Some(&self.0.values[self.0.record_size * self.1 + column_index]))
        } else {
            OptionalValue(None)
        }
    }
}

#[derive(Debug)]
pub enum Error {
    FileNotFound,
    UnknownColumnType(String, String),
    InvalidColumnType,
    UnknownColumnName,
    OutOfBounds(usize),
    DecodingError,
}

impl PSV {
    pub fn from_file<P>(path : P) -> Result<PSV, Error> where P : AsRef<Path> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(_) => return Err(Error::FileNotFound)
        };

        Self::new(file)
    }

    pub fn new<R>(source : R) -> Result<PSV, Error> where R : Read {
        // Maybe rework this to store the entire buffer as one giant block
        // ... meaning Value would wrap a Range<usize> over said buffer
        let lines = BufReader::new(source)
            .lines();
    
        let mut columns = Vec::<(String, Type)>::new();
        let mut values = Vec::<Value>::new();
        let mut record_size = 0;

        let mut i = 0;
        for line in lines {
            let line = line.unwrap();
            if line.starts_with('#') { continue; }

            let tokens : Vec<_> = line.split('|').collect();

            if i == 0 {
                record_size = tokens.len();
                for token in tokens {
                    let column_type_marker = token.chars()
                        .position(|c| c == '!')
                        .unwrap_or(token.len());
                    let column_size_marker = column_type_marker + token.chars()
                        .skip(column_type_marker)
                        .position(|c| c == ':')
                        .unwrap_or(token.len());
                    let column_name = token[0..column_type_marker].to_owned();
                    let column_type = &token[column_type_marker + 1..column_size_marker];
                    let column_size = token[column_size_marker + 1..].parse::<usize>().unwrap_or(0);

                    let column_type = match column_type {
                        "DEC" => Type::Dec(column_size),
                        "HEX" => Type::Hex(column_size),
                        "STRING" => Type::String,
                        _ => return Err(Error::UnknownColumnType(column_name, column_type.to_owned()))
                    };

                    columns.push((column_name, column_type));
                }
            } else {
                for (i, token) in tokens.into_iter().enumerate() {
                    values.push(Value(token.to_owned(), columns[i].1));
                }
            }

            i += 1;
        }

        columns.shrink_to_fit();
        values.shrink_to_fit();

        Ok(Self {
            columns,
            record_size,
            values,
        })
    }

    pub fn for_each_record<F>(&self, callback : F) where F : FnMut(Record) {
        (0..self.record_count()).map(|i| Record(&self, i)).for_each(callback)
    }

    pub fn record_count(&self) -> usize {
        self.values.len() / self.record_size
    }

    pub fn record(&self, index : usize) -> Option<Record> {
        if index * self.record_size >= self.values.len() {
            None
        } else {
            Some(Record(&self, index))
        }
    }
}

#[cfg(test)]
mod test {
    use super::PSV;

    #[test]
    pub fn parser() {
        let buffer = include_bytes!("../../tests/build.info.sample");

        let psv = PSV::new(&buffer[..]).expect("This should be a valid file");

        assert_eq!(psv.record_count(), 1);
        psv.for_each_record(|record| {
            let branch = record.read("Branch").try_string()
                .expect("Should never happen");
            let product = record.read("Product").try_string()
                .expect("Should never happen");
            let active = record.read("Active").try_bool()
                .expect("Should never happen");
            let cdn_hosts = record.read("CDN Hosts").try_strings()
                .expect("Should never happen");
            let build_key = record.read("Build Key").try_hex(false)
                .expect("Should never happen");

            assert_eq!(branch, "eu");
            assert_eq!(product, "wow_classic");
            assert_eq!(active, true);
            assert_eq!(cdn_hosts, vec![ "blzddist1-a.akamaihd.net", "level3.blizzard.com", "eu.cdn.blizzard.com" ]);
            assert_eq!(build_key, vec![ 0x8A, 0xED, 0xE3, 0xC9, 0x2D, 0x9C, 0x28, 0xD8, 0x94, 0xCB, 0xC9, 0x78, 0xC5, 0xB7, 0xC2, 0x42 ]);
        });
    }
}