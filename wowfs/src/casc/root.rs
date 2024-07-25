use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::Range;
use bytes::Buf;
use crate::casc::types::ContentKey;

/// A World of Warcraft "Root" file, as specified by build-configuration files. This file associates
/// [`ContentKey`]s with [`FileDataID`], and optionally one or many name hashes. These hashes are
/// Jenkins hashes of the file's complete path in the game's directory structure.
pub struct Root {
    pages: Vec<Page>,
    hashes: HashMap<u64, (usize, usize)>,
}
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

        let allow_non_named_files = match format {
            Format::Legacy => false,
            Format::MSFT { total_file_count, named_file_count, .. } => {
                total_file_count != named_file_count
            }
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
                    let increment = cursor.get_i32_le();
                    assert!(increment >= 0, "Negative fdid increment");

                    fdid += increment + 1;
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
                Format::MSFT { .. } => {
                    let ckr : Range<usize> = Range {
                        start: 0,
                        end: record_count * 16
                    };
                    let nhr = Range {
                        start : ckr.end,
                        end: ckr.end + if !(allow_non_named_files && (content_flags & 0x10000000) != 0) {
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
            pages.sort_by_key(|p| p.records[0].fdid());

            // Collect hash associative map
            let hashes : HashMap<u64, (usize, usize)> = pages
                .iter()
                .enumerate()
                .filter_map(|(pg_idx, page)| {
                    if !(allow_non_named_files && (page.content_flags & 0x10000000) != 0) {
                        None
                    } else {
                        page.records
                            .iter()
                            .enumerate()
                            .map(move |(rec_idx, rec)| {
                                (rec.name_hash(), (pg_idx, rec_idx))
                            })
                            .into()
                    }
                })
                .flatten()
                .collect();

            Some(Root {
                pages,
                hashes
            })
        }
    }

    /// Finds a record in the file for a given [`FileDataID`].
    ///
    /// # Arguments
    ///
    /// * `fdid` - The file data ID to look for. This ID is unique for the file.
    pub fn find_fdid(&self, fdid: u64) -> Option<&Record> {
        self.pages.binary_search_by(|page| {
            let range = Range {
                start: page.records[0].fdid(),
                end: page.records[page.records.len() - 1].fdid() + 1 // End is exclusive
            };

            if range.contains(&fdid) {
                Ordering::Equal
            } else if range.start > fdid {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }).and_then(|pg_idx| {
            let page = &self.pages[pg_idx];

            page.records
                .binary_search_by(|record| record.fdid().cmp(&fdid))
                .map(|rec_idx| (rec_idx, pg_idx))
        }).map(|(rec_idx, pg_idx)| {
            &self.pages[pg_idx].records[rec_idx]
        }).ok()
    }

    /// Finds the given file hash in this structure.
    ///
    /// # Arguments
    ///
    /// * `hash` - The Jenkins hash of the file path.
    pub fn find_hash(&self, hash : u64) -> Option<&Record> {
        self.hashes.get(&hash).map(|(pg_idx, rec_idx)| {
            &(self.pages[*pg_idx].records[*rec_idx])
        })
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
