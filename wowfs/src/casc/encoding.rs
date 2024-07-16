use std::ops::Range;
use std::collections::HashMap;
use std::io::Read;
use bitmask_enum::bitmask;
use byteorder::{BigEndian, ReadBytesExt};
use bytes::Buf;
use crate::casc::errors::EncodingError;

use super::errors::Error;
use super::types::{ContentKey, EncodingKey};

pub struct Encoding {
    especs : Vec<String>,
    content_map : HashMap<ContentKey, Entry>,
    encoding_map : HashMap<EncodingKey, (usize, u64)>,
    espec : String
}

#[bitmask(u8)]
pub enum EncodingLoadFlags {
    Content,
    Encoding,
    EncodingSpec,
}

impl Encoding {
    pub fn new(data : &[u8], flags : EncodingLoadFlags) -> Result<Encoding, Error> {
        let mut cursor = data;
        if cursor.remaining() < 0x16 { return Err(EncodingError::Malformed.into()); }
        if &cursor[0..2] != b"EN"  { return Err(EncodingError::InvalidSignature.into()); }
        if cursor[2] != 1  { return Err(EncodingError::InvalidVersion.into()); }
        cursor.advance(2 + 1);

        let header = match Self::read_header(&mut cursor) {
            Ok(header) => header,
            Err(err) => return Err(err.into())
        };

        let especs = {
            if flags.contains(EncodingLoadFlags::EncodingSpec) {
                cursor[0..header.espec].split(|byte| *byte == 0)
                    .filter_map(|bytes| {
                        match String::from_utf8(bytes.to_vec()) {
                            Ok(v) => Some(v),
                            Err(_) => None,
                        }
                    })
                    .collect::<Vec<_>>()
            } else {
                vec![]
            }
        }; cursor.advance(header.espec);

        let mut content_map = HashMap::<ContentKey, Entry>::new();

        if flags.contains(EncodingLoadFlags::Content) {
            let pages_sz = header.content.page_count * (header.content.key_size + 0x10 + header.content.page_size);

            let mut section = Vec::with_capacity(pages_sz);
            section.resize(pages_sz, 0);
            match cursor.read_exact(&mut section) {
                Ok(_) => (),
                Err(_) => return Err(EncodingError::Malformed.into()),
            };

            for (ckr, hr, pr) in (0..header.content.page_count).map(|i| {
                let page_header = i * (header.content.key_size + 0x10);
                let content_key = Range {
                    start : page_header,
                    end : page_header + header.content.key_size
                };

                let hash = Range {
                    start : content_key.end,
                    end : content_key.end + 0x10
                };

                // Skip over all header checksum blocks
                let page_data = header.content.page_count * (header.content.key_size + 0x10) + i * header.content.page_size;
                let page = Range {
                    start : page_data,
                    end : page_data + header.content.page_size
                };

                (content_key, hash, page)
            }) {
                let first_ckey = ContentKey::from(&section[ckr]);
                let hash = (&section[hr]).read_u128::<BigEndian>().unwrap();
                let page_range = &section[pr];
                let mut page_data = page_range.chunk();

                let page_hash = u128::from_be_bytes(*md5::compute(page_data));
                assert_eq!(page_hash, hash);

                let mut first = true;
                while page_data.remaining() >= (1 + 5 + header.content.key_size) && page_data[0] != 0x00 {
                    let key_count = page_data.get_u8() as usize;
                    let file_size = page_data.get_uint(5);
                    let content_key = ContentKey::from(&page_data[0..header.content.key_size]);

                    assert!(!first || content_key == first_ckey);
                    first = false;

                    page_data.advance(header.content.key_size);

                    let mut keys = Vec::<EncodingKey>::with_capacity(key_count);
                    page_data.chunks(header.encoding.key_size)
                        .take(key_count)
                        .map(&EncodingKey::from)
                        .for_each(|k| keys.push(k));

                    page_data.advance(key_count * header.encoding.key_size);

                    content_map.insert(content_key, Entry { keys, file_size });
                }
            }
        } else {
            cursor.advance(header.content.page_count * (header.content.key_size + 0x10 + header.content.page_size));
        }

        let mut encoding_map = HashMap::<EncodingKey, (usize, u64)>::new();
        if flags.contains(EncodingLoadFlags::Encoding) {
            let pages_sz = header.encoding.page_count * (header.encoding.key_size + 0x10 + header.encoding.page_size);

            let mut section = Vec::with_capacity(pages_sz);
            section.resize(pages_sz, 0);
            match cursor.read_exact(&mut section) {
                Ok(_) => (),
                Err(_) => return Err(EncodingError::Malformed.into()),
            };

            for (ekr, hr, pr) in (0..header.encoding.page_count).map(|i| {
                let page_header = i * (header.encoding.key_size + 0x10);
                let encoding_key = Range {
                    start : page_header,
                    end : page_header + header.encoding.key_size
                };

                let hash = Range {
                    start : encoding_key.end,
                    end : encoding_key.end + 0x10
                };

                // Skip over all header checksum blocks
                let page_data = header.encoding.page_count * (header.encoding.key_size + 0x10) + i * header.encoding.page_size;
                let page = Range {
                    start : page_data,
                    end : page_data + header.encoding.page_size
                };

                (encoding_key, hash, page)
            }) {
                let first_ekey = EncodingKey::from(&section[ekr]);
                let hash = (&section[hr]).read_u128::<BigEndian>().unwrap();
                let page_range = &section[pr];
                let mut page_data = page_range.chunk();

                let page_hash = u128::from_be_bytes(*md5::compute(page_data));
                assert_eq!(page_hash, hash);

                let mut first = true;
                while page_data.remaining() >= (4 + 5 + header.encoding.key_size) && page_data[0] != 0x00 {
                    let encoding_key = EncodingKey::from(&page_data[0..header.encoding.key_size]);
                    page_data.advance(header.encoding.key_size);

                    let index = page_data.get_u32() as usize;
                    let file_size = page_data.get_uint(5);

                    assert!(!first || encoding_key == first_ekey);
                    first = false;

                    encoding_map.insert(encoding_key, (index, file_size));
                }
            }
        } else {
            cursor.advance(header.encoding.page_count * (header.encoding.key_size + 0x10 + header.encoding.page_size));
        }

        // Trailing data left: espec of this file itself
        let espec = match String::from_utf8(cursor.to_vec()) {
            Ok(spec) => spec,
            Err(_) => return Err(EncodingError::Malformed.into())
        };

        cursor.advance(espec.as_bytes().len());

        assert!(!cursor.has_remaining());

        Ok(Encoding {
            especs,
            content_map,
            encoding_map,
            espec
        })
    }

    fn read_header<R>(source : &mut R) -> Result<Header, EncodingError> where R : Read + Buf {
        if source.remaining() < (1 + 1 + 2 + 2 + 4 + 4 + 1 + 4) {
            return Err(EncodingError::Malformed);
        }

        let ckey_size = source.get_u8();
        let ekey_size = source.get_u8();
        let cpage_size = usize::from(source.get_u16()) * 1024;
        let epage_size = usize::from(source.get_u16()) * 1024;
        let ccount = source.get_u32();
        let ecount = source.get_u32();

        let _unknown = source.get_u8();
        let espec_size = source.get_u32();

        Ok(Header {
            encoding : Spec {
                key_size : ekey_size as _,
                page_size: epage_size,
                page_count : ecount as _
            },
            content : Spec {
                key_size : ckey_size as _,
                page_size: cpage_size,
                page_count : ccount as _
            },
            espec : espec_size as _
        })
    }

    pub fn find(&self, key : &ContentKey) -> Option<&Entry> {
        self.content_map.get(key)
    }
}

pub struct Entry {
    pub keys : Vec<EncodingKey>,
    pub file_size : u64
}

struct Header {
    content : Spec,
    encoding : Spec,
    espec : usize
}

struct Spec {
    key_size : usize,
    page_size : usize,
    page_count : usize
}