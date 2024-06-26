use std::path::{Path, PathBuf};

use crate::file_formats::config::Config;
use crate::file_formats::config::specs::{self as specs, Spec};
use crate::tact::encoding::Encoding;

pub struct FileSystem {
    path : PathBuf,
    build : (String, Config),
    cdn : (String, Config),
}
impl FileSystem {
    pub fn open<P>(path : P, build : &str, cdn : &str) -> Result<FileSystem, Error> where P : AsRef<Path> {
        let build = match Config::from_file(path.as_ref().join(format!("/Data/config/{}/{}/{}", &build[0..2], &build[2..4], build))) {
            Ok(file) => (build.to_owned(), file),
            Err(_) => return Err(Error::FileNotFound(build.to_owned()))
        };

        let cdn = match Config::from_file(path.as_ref().join(format!("/Data/config/{}/{}/{}", &cdn[0..2], &cdn[2..4], cdn))) {
            Ok(file) => (cdn.to_owned(), file),
            Err(_) => return Err(Error::FileNotFound(cdn.to_owned()))
        };

        Ok(Self {
            path : path.as_ref().to_path_buf(),
            build,
            cdn,
        })
    }
}

pub enum Error {
    FileNotFound(String)
}

pub struct FileSystemProvider;
impl FileSystemProvider {
    pub fn enumerate(root : PathBuf) {

    }
}