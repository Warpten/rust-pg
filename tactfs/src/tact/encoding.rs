use std::collections::HashMap;

use anyhow::{ensure, Context, Result};
use bytes::Buf;
use enumflags2::{bitflags, BitFlags};
use thiserror::Error;

use super::types::{ContentKey, EncodingKey};

pub(crate) struct Encoding {
    especs : Vec<String>,
    content_map : HashMap<ContentKey, (Vec<EncodingKey>, u64)>,
    encoding_map : HashMap<u128, (usize, u64)>,
    espec : String
}

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum LoadFlags {
    Content = 0x01,
    Encoding = 0x02,
    EncodingSpec = 0x04,
}

#[derive(Error, Debug)]
pub enum ErrorCode {
    #[error("Truncated header")]
    TruncatedHeader,
    #[error("Not an encoding spec")]
    NotEncoding,
    #[error("Invalid encoding version : found {0}, expected 1")]
    InvalidVersion(u8),
    #[error("Unexpected value at {0}: found {1}, expected {2}")]
    Unexpected(usize, u8, u8),
    #[error("Truncated encoding-spec table: expected {0}, found {1}")]
    TruncatedEspec(usize, usize),
    #[error("Invalid hash")]
    InvalidHash
}

impl Encoding {
    pub(crate) fn new(data : &[u8], flags : BitFlags<LoadFlags>) -> Result<Encoding> {
        let mut cursor = data;
        ensure!(cursor.remaining() >= 16, ErrorCode::TruncatedHeader);
        ensure!(&cursor[0..2] == b"NE", ErrorCode::NotEncoding);
        ensure!(cursor[2] == 1, ErrorCode::InvalidVersion(cursor[2]));
        cursor.advance(2 + 1);

        let ckey_size : usize = cursor.get_u8().into();
        let ekey_size : usize = cursor.get_u8().into();

        let cpage_size : usize = usize::from(cursor.get_u16()) * 1024;
        let epage_size : usize = usize::from(cursor.get_u16()) * 1024;

        let ccount : usize = cursor.get_u32().try_into()?;
        let ecount : usize = cursor.get_u32().try_into()?;

        ensure!(cursor[0] == 0, ErrorCode::Unexpected(2 + 3 + 2 + 2 + 4 + 4, cursor[0], 0));

        let espec_size = cursor.get_u32().try_into()?;
        ensure!(cursor.remaining() >= espec_size, ErrorCode::TruncatedEspec(espec_size, cursor.remaining()));

        let especs = {
            if flags.contains(LoadFlags::EncodingSpec) {
                cursor[0..espec_size].split(|byte| *byte == 0)
                    .map(|bytes| String::from_utf8(bytes.to_vec()).context("Parsing encoding spec"))
                    .collect::<Result<Vec<String>>>()?
            } else {
                vec![]
            }
        }; cursor.advance(espec_size);

        ensure!(cursor.remaining() >= ccount * 32);
        
        let mut content_map = HashMap::<ContentKey, (Vec<EncodingKey>, u64)>::new();

        let mut content_pages = Vec::<(ContentKey, u128)>::with_capacity(ccount);
        if flags.contains(LoadFlags::Content) {
            for _itr in 0..ccount {
                let content_key = ContentKey::new(&cursor[0..ckey_size]);
                cursor.advance(ckey_size);

                let hash = cursor.get_u128();

                content_pages.push((content_key, hash));
            }

            for (first_key, hash) in content_pages {
                // This is cursed; does it even work?
                let runtime_hash = u128::from_be_bytes(md5::compute(&cursor[0..cpage_size]).0);

                ensure!(hash == runtime_hash, ErrorCode::InvalidHash);

                let mut page = Buf::take(cursor, cpage_size);
                let mut first = true;

                while page.remaining() >= (1 + 5 + ckey_size) && page.chunk()[0] != 0x00 {
                    let key_count = usize::from(page.get_u8());
                    let file_size = (u64::from(page.get_u8()) << 32) | u64::from(page.get_u32());
                    let content_key = ContentKey::from(&mut page, ckey_size, true);

                    ensure!(!first || first_key == content_key); // First key mismatch in content
                    first = false;

                    ensure!(page.remaining() >= key_count * 16_usize);

                    let mut encoding_keys = Vec::<EncodingKey>::with_capacity(key_count);
                    page.chunk().chunks(ekey_size)
                        .map(&EncodingKey::new)
                        .for_each(|k| encoding_keys.push(k));

                    page.advance(key_count * ekey_size);
                    
                    content_map.insert(content_key, (encoding_keys, file_size));
                }
                cursor.advance(cpage_size);
            }
        } else {
            cursor.advance(ccount * (ckey_size + 0x10 + cpage_size));
        }

        Ok(Encoding {
            especs,
            content_map,
            encoding_map : HashMap::new(),
            espec : "".to_string()
        })
    }
}