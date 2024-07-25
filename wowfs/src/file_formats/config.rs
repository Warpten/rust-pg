use std::borrow::Borrow;
use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::path::Path;

pub struct Config {
    values : HashMap<String, String>,
}

impl Config {
    pub fn from_file<P>(path : P) -> Result<Config, Error> where P : AsRef<Path> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(_) => return Err(Error::FileNotFound)
        };

        Self::new(file)
    }

    pub fn new<R>(source : R) -> Result<Config, Error> where R : Read {
        // Maybe rework this to store the entire buffer as one giant block
        // ... meaning Value would wrap a Range<usize> over said buffer
        let lines = BufReader::new(source)
            .lines();

        let mut values = HashMap::new();
        for line in lines {
            let line = line.unwrap();

            if line.starts_with('#') { continue; }

            if let Some((left, right)) = line.split_once(" = ") {
                values.insert(left.to_owned(), right.to_owned());
            }
        }

        values.shrink_to_fit();

        Ok(Self { values })
    }
}

pub mod specs {
    use crate::casc::types::{ContentKey, EncodingKey};

    pub trait Spec<'a> {
        type Value;

        fn read(source : &'a super::Config) -> Self::Value;
    }

    macro_rules! specs {
        ($t:tt, $x:literal, RegularExpression) => {
            pub struct $t;
            impl<'a> Spec<'a> for $t {
                type Value = std::collections::HashMap<&'a str, &'a str>;

                fn read(source: &'a super::Config) -> Self::Value {
                    source.values.iter()
                        .flat_map(|(k, v)| {
                            k.matches($x).map(|arg| {
                                (arg, v.as_str())
                            })
                        })
                        .collect()
                }
            }
        };
        ($t:tt, $x:literal, Pair) => {
            pub struct $t;
            impl<'a> Spec<'a> for $t {
                type Value = (&'a str, &'a str);

                fn read(source : &'a super::Config) -> Self::Value {
                    source.values.get($x).unwrap().split_once(' ').unwrap()
                }
            }
        };
        ($t:tt, $x:literal, Pair, $l:tt, $r:tt) => {
            pub struct $t;
            impl<'a> Spec<'a> for $t {
                type Value = ($l, $r);

                fn read(source : &'a super::Config) -> Self::Value {
                    let vals = source.values.get($x).unwrap().split_once(' ').unwrap();
                    ($l::from(vals.0), $r::from(vals.1))
                }
            }
        };
        ($t:tt, $x:literal, Vec) => {
            pub struct $t;
            impl<'a> Spec<'a> for $t {
                type Value = Vec<&'a str>;

                fn read(source : &'a super::Config) -> Self::Value {
                    source.values.get($x).unwrap().split(' ').collect()
                }
            }
        };
        ($t:tt, $x:literal, ContentKey) => {
            pub struct $t;
            impl<'a> Spec<'a> for $t {
                type Value = ContentKey;

                fn read(source : &'a super::Config) -> Self::Value {
                    From::<&str>::from(source.values.get($x).unwrap())
                }
            }
        };
    }

    specs! { EncodingSpec, "encoding", Pair, ContentKey, EncodingKey }
    specs! { RootSpec, "root", ContentKey }
    specs! { ArchivesSpec, "archives", Vec }
    specs! { KeyringSpec, "key-([a-f0-9]+)", RegularExpression }
}

#[derive(Debug)]
pub enum Error {
    FileNotFound,
}