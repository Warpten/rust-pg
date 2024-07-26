use std::{fs::File, io::{BufRead, BufReader, Read, Seek, SeekFrom}, ops::Deref, path::Path};
use std::fmt::Debug;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;
use rust_salsa20::{Key, Salsa20};
use salsa20::{cipher::KeyIvInit, SalsaCore};

use crate::crypto::RC4;

use super::{errors::Error, keyring::KeyRing};

struct ChunkInfo {
    compressed_size : u32,
    decompressed_size : u32,
    checksum : u128,
}

pub struct Spec {
    flags : u8,
    chunks : Vec<ChunkInfo>,
}

pub struct BLTE(Vec<u8>, Spec);
impl BLTE {
    pub fn new<P>(keyring: &KeyRing, archive : P, offset : u64, length : u64) -> Result<Self, Error> where P : AsRef<Path> {
        Self::new_with_buf(archive, offset, length, Vec::new(), keyring)
    }

    pub fn from<R>(source : &mut R, dest : &mut Vec<u8>, keyring: &KeyRing) -> Result<Self, Error> where R : ReadBytesExt + BufRead + Debug {
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

            let checksum = source.read_u128::<BigEndian>().unwrap();

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
                    let mut section_start = dest.len();
                    match section.read_to_end(dest) {
                        Ok(size) => {
                            // These are broken don't ask me why
                            // let hash = u128::from_be_bytes(*md5::compute(&dest[section_start..section_start + size]));
                            // assert_eq!(hash, chunk.checksum);
                        },
                        Err(_) => todo!(),
                    }
                },
                b'Z' => {
                    let mut compressed_data = Vec::with_capacity(chunk.compressed_size as _);
                    match section.read_to_end(&mut compressed_data) {
                        Ok(value) if value as u32 == chunk.compressed_size => (),
                        _ => return Err(Error::MalformedArchive)
                    };

                    // These are broken don't ask me why
                    // assert_eq!(chunk.checksum, u128::from_be_bytes(*md5::compute(&compressed_data)));

                    match ZlibDecoder::new(&compressed_data[..]).read_to_end(dest) {
                        Ok(read_count) if read_count as u32 == chunk.decompressed_size => (),
                        _ => return Err(Error::MalformedArchive)
                    };
                }
                b'4' => {
                    unimplemented!("lz4hc not implemented");
                }
                b'E' => {
                    let key = if let Ok(key_name_sz) = section.read_u8() {
                        let mut key = Vec::with_capacity(key_name_sz as _);
                        key.resize(key_name_sz as _, 0);

                        let key = match section.read(&mut key) {
                            Ok(sz) if sz == (key_name_sz as usize) => keyring.try_get(key),
                            _ => return Err(Error::MalformedArchive)
                        };

                        if let Some(key) = key {
                            key
                        } else {
                            return Err(Error::MalformedArchive);
                        }
                    } else { 
                        return Err(Error::MalformedArchive);
                    };

                    let iv = if let Ok(iv_len) = section.read_u8() {
                        let mut iv = Vec::with_capacity(iv_len as _);
                        iv.resize(iv_len as _, 0);

                        match section.read(&mut iv) {
                            Ok(sz) if sz == (iv_len as usize) => iv,
                            _ => return Err(Error::MalformedArchive)
                        }
                    } else {
                        return Err(Error::MalformedArchive);
                    };
                    
                    if let Ok(r#type) = section.read_u8() {
                        match r#type {
                            b'A' => {
                                let mut state = RC4::new(key);

                                let data_offset = dest.len();
                                section.read_to_end(dest);
                                state.update(&mut dest[data_offset..]);
                            },
                            b'S' => {
                                let mut state = Salsa20::new(
                                    Key::Key16(*key),
                                    std::array::from_fn(|i| iv.get(i).copied().unwrap_or(0_u8)),
                                    0
                                );

                                let data_offset = dest.len();
                                section.read_to_end(dest);
                                state.encrypt(&mut dest[data_offset..]);
                            },
                            _ => return Err(Error::MalformedArchive)
                        }
                    } else {
                        return Err(Error::MalformedArchive);
                    }
                }
                b'F' => {
                    match Self::from(source, dest, keyring) {
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

    pub fn new_with_buf<P>(archive : P, offset : u64, length : u64, mut dest : Vec<u8>, keyring: &KeyRing) -> Result<Self, Error> where P : AsRef<Path> {
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

        Self::from(&mut section, &mut dest, keyring)
    }

    pub fn bytes(self) -> Vec<u8> { self.0 }
}

impl Deref for BLTE {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}