use std::{borrow::Borrow, collections::HashMap, fs::File, hash::Hash, io::{BufRead, BufReader, Read}, ops::Range, path::Path, slice::Iter};

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
    pub trait Spec<'a> {
        type Value;

        fn read(source : &'a super::Config) -> Self::Value;
    }

    enum SpecKind {
        Pair,
        Vec
    }

    macro_rules! specs {
        ($t:tt, $x:literal, SpecKind::Pair) => {
            pub struct $t;
            impl<'a> Spec<'a> for $t {
                type Value = (&'a str, &'a str);

                fn read(source : &'a super::Config) -> Self::Value {
                    source.values.get($x).unwrap().split_once(' ').unwrap()
                }
            }
        };
        ($t:tt, $x:literal, SpecKind::Vec) => {
            pub struct $t;
            impl<'a> Spec<'a> for $t {
                type Value = Vec<&'a str>;

                fn read(source : &'a super::Config) -> Self::Value {
                    source.values.get($x).unwrap().split(' ').collect()
                }
            }
        }
    }

    specs! { Encoding, "encoding", SpecKind::Pair }
    specs! { Root, "root", SpecKind::Pair }
    specs! { Archives, "archives", SpecKind::Vec }
}

#[derive(Debug)]
pub enum Error {
    FileNotFound,
}
