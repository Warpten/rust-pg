use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    FileNotFound(PathBuf),
    EncodingNotFound(String),
    RootNotFound(String),
    ReadError,
    MalformedArchive,
    Encrypted,

    Encoding(EncodingError)
}

#[derive(Debug)]
pub enum EncodingError {
    Malformed,
    InvalidSignature,
    InvalidVersion,
}

impl Into<Error> for EncodingError {
    fn into(self) -> Error {
        Error::Encoding(self)
    }
}