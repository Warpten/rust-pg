use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use crate::casc::encoding::{self, Encoding, EncodingLoadFlags};
use crate::casc::types::{ContentKey, EncodingKey};
use crate::file_formats::config::Config;
use crate::file_formats::config::specs::{EncodingSpec, Spec};
use crate::casc::errors::Error;
use crate::casc::index::{Entry, Index};

pub struct FileSystem {
    path : PathBuf,
    build : (PathBuf, Config),
    cdn : (PathBuf, Config),
    indices : Vec<Index>,
    encoding : Encoding,
}
impl FileSystem {
    pub fn open<P>(path : P, build : &str, cdn : &str) -> Result<FileSystem, Error> where P : AsRef<Path> {
        let build = path.as_ref().join(format!("Data/config/{}/{}/{}", &build[0..2], &build[2..4], build));
        let build = match Config::from_file(&build) {
            Ok(file) => (build, file),
            Err(_) => return Err(Error::FileNotFound(PathBuf::from(build)))
        };

        let cdn = path.as_ref().join(format!("Data/config/{}/{}/{}", &cdn[0..2], &cdn[2..4], cdn));
        let cdn = match Config::from_file(&cdn) {
            Ok(file) => (cdn, file),
            Err(_) => return Err(Error::FileNotFound(PathBuf::from(cdn)))
        };

        let mut indices : Vec<_> = match std::fs::read_dir(path.as_ref().join("Data/data")) {
            Ok(data) => data.filter_map(Result::ok)
                .map(|d| d.path())
                .filter(|p| p.extension().map_or(false, |ext| ext == "idx"))
                .map(|path| Index::new(path))
                .filter_map(Result::ok)
                .collect(),
            Err(_) => return Err(Error::FileNotFound(PathBuf::from("Data/data")))
        };

        indices.sort_by(|l,r| l.bucket().cmp(&r.bucket()));

        // Now that indices are loaded, find encoding.
        let encoding_keys = EncodingSpec::read(&build.1);
        let encoding = find_ekey(&indices, &encoding_keys.1)
            .into_iter()
            .inspect(|e| println!("{:?}", e))
            .find_map(|e| {
                match e.read()
                    .and_then(|file| Encoding::new(&file.bytes(), EncodingLoadFlags::Content)) {
                    Ok(encoding) => Some(encoding),
                    Err(_) => None,
                }
            });

        if encoding.is_none() {
            return Err(Error::EncodingNotFound(encoding_keys.1.to_string()));
        }

        Ok(Self {
            path : path.as_ref().to_path_buf(),
            encoding : encoding.unwrap(),
            build,
            cdn,
            indices,
        })
    }

    /// Searches for an encoding key, returning a slice of entries in the indices that match that key.
    /// 
    /// # Arguments
    /// 
    /// * `key` - The encoding key to search for.
    pub fn find_encoding(&self, key : &EncodingKey) -> Vec<Entry> {
        find_ekey(&self.indices, key)
    }

    /// Searches for a content key, returning a set of entries in data indices that match the associated encoding key.
    ///
    /// # Arguments
    ///
    /// * `key` - The content key to search for.
    pub fn find_content(&self, key : &ContentKey) -> Vec<Entry> {
        self.encoding.find(key)
            .iter()
            .flat_map(|entry| {
                entry.keys
                    .iter()
                    .flat_map(|key| self.find_encoding(key))
            })
            .collect()
    }
}

fn find_ekey<'a>(indices : &'a[Index], key : &EncodingKey) -> Vec<Entry<'a>>
{
    // Bucket index
    let bucket_index = key[0] ^ key[1] ^ key[2] ^ key[3] ^ key[4] ^ key[5] ^ key[6] ^ key[7] ^ key[8];
    let bucket_index = (bucket_index & 0xF) ^ (bucket_index >> 4);
    assert!((bucket_index as usize) < indices.len());

    let index = &indices[bucket_index as usize];

    // Construct all entries
    let entries : Vec<_> = (0..index.entry_count())
        .filter_map(|i| index.entry(i as _))
        .collect();

    let lookup = &key[index.spec.key()];

    // Entries in the file are sorted and there can be multiple entries for a single key
    let lower_bound = entries.binary_search_by(|entry| {
        match entry.key().cmp(&lookup) {
            Ordering::Equal => Ordering::Greater,
            ord => ord,
        }
    }).unwrap_err();

    let upper_bound = entries.binary_search_by(|entry| {
        match entry.key().cmp(&lookup) {
            Ordering::Equal => Ordering::Less,
            ord => ord
        }
    }).unwrap_err();

    (lower_bound..upper_bound).filter_map(|i| index.entry(i)).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    pub fn open_fs() {
        let fs = super::FileSystem::open("D:/01 - Games/World of Warcraft/", "3fca2ca5b5b1d2195b09b6ff4181410d", "1362038d83a2fd77738d50befc33e4f2")
            .expect("Failed to open fs");

    }
}