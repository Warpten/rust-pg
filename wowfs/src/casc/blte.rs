use std::{fs::File, io::{BufRead, BufReader, Read, Seek, SeekFrom}, ops::{Deref, Range}, path::Path};
use std::fmt::Debug;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use bytes::Buf;
use flate2::read::ZlibDecoder;

use super::errors::Error;

struct ChunkInfo {
    compressed_size : u32,
    decompressed_size : u32,
    checksum : [u8; 16],
}

pub struct Spec {
    flags : u8,
    chunks : Vec<ChunkInfo>,
}

pub struct BLTE(Vec<u8>, Spec);
impl BLTE {
    pub fn new<P>(archive : P, offset : u64, length : u64) -> Result<Self, Error> where P : AsRef<Path> {
        Self::new_with_buf(archive, offset, length, Vec::new())
    }

    pub fn from<R>(source : &mut R, dest : &mut Vec<u8>) -> Result<Self, Error> where R : ReadBytesExt + BufRead + Debug {
        // Magic
        match source.read_u32::<LittleEndian>() {
            Ok(value) if value == 0x45544C42 => (),
            ctch => {
                println!("{:?} {:?}", source, ctch);
                return Err(Error::MalformedArchive)
            }
        };

        // Header Size
        match source.read_u32::<BigEndian>() {
            Ok(_) => {
                /* wtf do, don't have limit() here to see how much shit remains in R */
            }
            _ => return Err(Error::MalformedArchive)
        };

        let flags = match source.read_u8() {
            Ok(value) => value,
            Err(_) => return Err(Error::MalformedArchive)
        };
        let chunk_count = match source.read_u24::<BigEndian>() {
            Ok(value) => value,
            Err(_) => return Err(Error::MalformedArchive)
        };

        let chunks: Vec<_> = (0..chunk_count).filter_map(|_| {
            let compressed_size = source.read_u32::<BigEndian>().unwrap();
            let decompressed_size = source.read_u32::<BigEndian>().unwrap();

            let mut checksum = [0_u8; 16];
            _ = source.read_exact(&mut checksum);

            Some(ChunkInfo { compressed_size: compressed_size - 1, decompressed_size, checksum })
        }).collect();

        let allocation_size = chunks.iter().fold(0, |r, c| r + c.decompressed_size) as usize;
        if dest.capacity() < allocation_size {
            dest.reserve_exact(allocation_size - dest.capacity())
        }

        for chunk in &chunks {
            let encoding_mode = match source.read_u8() {
                Ok(value) => value,
                Err(_) => return Err(Error::MalformedArchive)
            };

            let mut section = source.take(chunk.compressed_size as _);

            match encoding_mode {
                b'N' => {
                    // _ = source.read_exact(&mut dest[section_range]);
                    _ = section.read_to_end(dest);
                },
                b'Z' => {
                    let mut compressed_data = Vec::with_capacity(chunk.compressed_size as _);
                    match section.read_to_end(&mut compressed_data) {
                        Ok(value) if value as u32 == chunk.compressed_size => (),
                        _ => return Err(Error::MalformedArchive)
                    };

                    match ZlibDecoder::new(&compressed_data[..]).read_to_end(dest) {
                        Ok(read_count) if read_count as u32 == chunk.decompressed_size => (),
                        _ => return Err(Error::MalformedArchive)
                    };
                }
                b'4' => {
                    unimplemented!("lz4hc not implemented");
                }
                b'E' => {
                    return Err(Error::Encrypted);
                }
                b'F' => {
                    match Self::from(source, dest) {
                        Ok(_) => (),
                        error => return error
                    };
                },
                _ => unreachable!("Unsupported chunk encoding {}", encoding_mode),
            };
        }

        dest.shrink_to_fit();
        Ok(Self(dest.to_vec(), Spec { flags, chunks }))
    }

    pub fn new_with_buf<P>(archive : P, offset : u64, length : u64, mut dest : Vec<u8>) -> Result<Self, Error> where P : AsRef<Path> {
        let mut reader = match File::open(&archive) {
            Ok(file) => BufReader::new(file),
            Err(_) => return Err(Error::FileNotFound(archive.as_ref().to_owned()))
        };

        match reader.seek(SeekFrom::Start(offset)) {
            Ok(_) => (),
            Err(_) => return Err(Error::MalformedArchive)
        };

        assert!(length >= 4 + 4 + 1 + 3);
        let mut section = reader.take(length);

        Self::from(&mut section, &mut dest)
    }

    pub fn bytes(self) -> Vec<u8> { self.0 }
}

impl Deref for BLTE {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}