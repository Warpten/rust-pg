use std::cmp::Ordering;
use std::ops::Range;
use bytes::Buf;
use crate::casc::types::ContentKey;

pub struct Root(Vec<Page>);

impl Root {
    pub fn read(source: &[u8]) -> Option<Root> {
        let mut cursor = source;
        let format = if cursor[..4] == *b"TSFM" {
            cursor.advance(4);

            let header_size = cursor.get_u32_le();
            let version = cursor.get_u32_le();
            if header_size > 1000 {
                Format::MSFT { version: u32::MIN, total_file_count: header_size, named_file_count: version }
            } else {
                let total_file_count = cursor.get_u32_le();
                let named_file_count = cursor.get_u32_le();
                _ = cursor.get_u32_le();

                Format::MSFT { version, total_file_count, named_file_count }
            }
        } else {
            Format::Legacy
        };

        let mut pages = Vec::<Page>::new();
        while cursor.has_remaining() {
            let record_count = cursor.get_u32_le() as usize;
            let content_flags = cursor.get_u32_le();
            let locale_flags = cursor.get_u32_le();

            if record_count == 0 {
                continue;
            }

            // Read all FDIDs - stored as deltas in the file so that we know FDIDs are sorted within a page.
            let mut fdids = Vec::<u64>::with_capacity(record_count);
            {
                let mut fdid = -1;
                for _ in 0..record_count {
                    // NOTE: This violently breaks if FDID delta goes negative even once.
                    fdid += cursor.get_i32_le() + 1;
                    fdids.push(fdid as _);
                }
            }

            let records = match format {
                Format::Legacy => {
                    let mut records = Vec::<Record>::with_capacity(record_count);
                    for i in 0..record_count {
                        let content_key = ContentKey::from(&cursor[0..16]);
                        cursor.advance(16);

                        let name_hash = cursor.get_u64_le();

                        records.push(Record(content_key, name_hash, fdids[i]));
                    }
                    records
                }
                Format::MSFT { total_file_count, named_file_count, .. } => {
                    let allow_non_named_files = (total_file_count != named_file_count) && (content_flags & 0x10000000) != 0;

                    let ckr : Range<usize> = Range {
                        start: 0,
                        end: record_count * 16
                    };
                    let nhr = Range {
                        start : ckr.end,
                        end: ckr.end + if !allow_non_named_files {
                            8 * record_count
                        } else {
                            0
                        }
                    };
                    let section_size = nhr.end;

                    let content_keys = &cursor[ckr];
                    let mut name_hashes = &cursor[nhr];
                    cursor.advance(section_size);

                    let mut records = Vec::<Record>::with_capacity(record_count);
                    for i in 0..record_count {
                        let content_key = ContentKey::from(&content_keys[Range {
                            start: i * 16,
                            end : (i + 1) * 16
                        }]);

                        let name_hash = if name_hashes.remaining() >= 8 {
                            name_hashes.get_u64_le()
                        } else {
                            0
                        };
                        records.push(Record(content_key, name_hash, fdids[i]));
                    }
                    records
                }
            };

            pages.push(Page {
                records,
                content_flags,
                locale_flags,
            });
        }

        #[cfg(debug_assertions)]
        match format {
            Format::MSFT { total_file_count, .. } => {
                assert_eq!(total_file_count as usize, pages.iter().fold(0, |s, p| s + p.records.len()))
            },
            _ => (),
        };

        if cursor.has_remaining() {
            None
        } else {
            pages.shrink_to_fit();
            Some(Root(pages))
        }
    }

    pub fn find_fdid(&self, fdid: u64) -> Option<&Record> {
        let lower_bound = self.0.binary_search_by(|page| {
            match page.records[0].fdid().cmp(&fdid) {
                Ordering::Equal => Ordering::Greater,
                ord => ord,
            }
        }).unwrap_err();

        let upper_bound = self.0.binary_search_by(|page| {
            match page.records[0].fdid().cmp(&fdid) {
                Ordering::Equal => Ordering::Less,
                ord => ord,
            }
        }).unwrap_err();

        if upper_bound == lower_bound {
            return None;
        }

        assert!(lower_bound + 1 == upper_bound);

        let page = &self.0[lower_bound];
        if let Ok(record_index) = page.records.binary_search_by(|record| record.fdid().cmp(&fdid)) {
            Some(&page.records[record_index])
        } else {
            None
        }
    }
}

enum Format {
    Legacy,
    MSFT { version: u32, total_file_count: u32, named_file_count: u32 },
}

struct Header {
    format: Format,
}

pub struct Record(ContentKey, u64, u64);
impl Record {
    pub fn content_key(&self) -> &ContentKey { &self.0 }
    pub fn name_hash(&self) -> u64 { self.1 }
    pub fn fdid(&self) -> u64 { self.2 }
}

struct Page {
    records: Vec<Record>,
    content_flags: u32,
    locale_flags: u32,
}
