use std::cmp::Ordering;
use std::io::Read;
use std::ops::Range;
use std::path::{Path, PathBuf};

use bytes::Buf;
use crate::casc::encoding::LoadFlags;
use crate::casc::types::EncodingKey;
use crate::file_formats::config::Config;
use crate::file_formats::config::specs::{Encoding, Spec};
use crate::casc::errors::Error;
use crate::casc::index::{Entry, Index};

pub struct FileSystem {
    path : PathBuf,
    build : (PathBuf, Config),
    cdn : (PathBuf, Config),
    indices : Vec<Index>,
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
        let encoding_keys = Encoding::read(&build.1);
        println!("({}, {})", encoding_keys.0, encoding_keys.1);
        let encoding = find_key(&indices, encoding_keys.1)
            .into_iter()
            .inspect(|e| println!("{:?}", e))
            .find_map(|e| {
                match e.read() {
                    Ok(file) => Some(file),
                    Err(_) => None,
                }
            })
            .map(|raw| super::casc::encoding::Encoding::new(&raw.bytes(), LoadFlags::Content | LoadFlags::EncodingSpec).unwrap());

        if encoding.is_none() {
            return Err(Error::EncodingNotFound(encoding_keys.0.to_string()));
        }

        Ok(Self {
            path : path.as_ref().to_path_buf(),
            build,
            cdn,
            indices,
        })
    }

    /// Searches for a key, returning a slice of entries in the indices that match that key.
    /// 
    /// # Arguments
    /// 
    /// * `key` - The key to search for.
    pub fn search<K>(&self, key : K) -> Vec<Entry> where K : Into<EncodingKey> {
        find_key(&self.indices, key)
    }
}

fn find_key<K>(indices : &[Index], key : K) -> Vec<Entry>
    where K : Into<EncodingKey>
{
    let key = key.into();

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
        let fs = super::FileSystem::open("D:/01 - Games/World of Warcraft/", "df750377895a52b1ce78afbc9a6d1bd6", "1c76ee368a486fd03befed99c1d14c76")
            .expect("Failed to open fs");

    }
}