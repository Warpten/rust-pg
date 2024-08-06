use std::hash::Hash;
use std::slice::SliceIndex;
use std::{borrow::Borrow, collections::HashMap};
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, Index, Range};
use crate::dbcd::{dbd::StructureDefinition, shared::RawValue};

pub struct Table {
    columns: HashMap<String, usize>,
    records: Vec<RawValue>,
    index_map: HashMap<u32, usize>,
}
impl Table {
    pub(in crate::dbcd) fn new(spec: &StructureDefinition, mut values: Vec<Vec<RawValue>>, ids: Vec<u32>) -> Table {
        let columns: HashMap<_, _> = spec.iter().map(|c| (c.name.to_string(), c.index)).collect();
        let linearized_values: Vec<_> = values.iter_mut().flat_map(|v| v.drain(..)).collect();

        Table {
            columns,
            records: linearized_values,
            index_map: ids.iter()
                .enumerate()
                .map(|(n, &id)| (id, n * spec.len()))
                .collect()
        }
    }

    /// Returns the record with the given ID, or an empty result if no record matches.
    /// 
    /// # Arguments
    /// 
    /// * `id` - The ID that is looked for.
    pub fn get(&self, id: u32) -> Option<Record> {
        if let Some(index) = self.index_map.get(&id) {
            let range = Range {
                start: *index,
                end: *index + self.columns.len(),
            };

            if range.start > self.records.len() && range.end > self.records.len() {
                None
            } else {
                let columns = &self.records[range];
                Some(Record {
                    table: self,
                    values: columns
                })
            }
        } else {
            None
        }
    }

    /// Returns an iterator over all rows of this table.
    #[inline]
    #[must_use]
    pub fn iter(&self) -> impl Iterator<Item = Record> {
        self.records.chunks(self.columns.len()).map(|values| {
            Record {
                table: self,
                values
            }
        })
    }

    /// Returns the amount of records in this table.
    pub fn len(&self) -> usize {
        self.records.len() / self.columns.len()
    }

    #[inline] pub fn column_count(&self) -> usize {
        self.columns.len()
    }
}

/// A DBC record providing accessors over columns based on their name.
pub struct Record<'a> {
    table: &'a Table,
    values: &'a [RawValue]
}

impl Debug for Record<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self.values)
    }
}

impl Deref for Record<'_> {
    type Target = [RawValue];

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl Index<&str> for Record<'_> {
    type Output = RawValue;

    fn index(&self, index: &str) -> &Self::Output {
        let column_index = self.table.columns[index];
        &self.values[column_index]
    }
}

impl Index<usize> for Record<'_> {
    type Output = RawValue;

    fn index(&self, index: usize) -> &Self::Output {
        &self.values[index]
    }
}

impl Record<'_> {
    /// Returns the value of a column for this record based on that column's name.
    /// If the name does not match any known column, returns an empty result.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of the column.
    #[inline]
    #[must_use]
    pub fn name<S>(&self, name: &S) -> Option<&RawValue> where String : Borrow<S>, S: Hash + Eq {
        self.table.columns.get(name).map(|&index| {
            &self.values[index]
        })
    }

    /// Returns a reference to a column or a subslice of columns depending on the type of
    /// index.
    ///
    /// - If given a position, returns a reference to the element at that
    ///   position or `None` if out of bounds.
    /// - If given a range, returns the subslice corresponding to that range,
    ///   or `None` if out of bounds.
    #[inline]
    #[must_use]
    pub fn at<I>(&self, index: I) -> Option<&<I as SliceIndex<[RawValue]>>::Output>
        where I : SliceIndex<[RawValue]>
    {
        self.values.get(index)
    }

    #[inline]
    pub fn len(&self) -> usize { self.values.len() }
}