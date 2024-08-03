use std::hash::Hash;
use std::slice::SliceIndex;
use std::{borrow::Borrow, collections::HashMap};
use std::ops::Index;
use crate::dbcd::{dbd::StructureDefinition, shared::RawValue};

pub struct Table {
    columns: HashMap<String, usize>,
    records: Vec<RawValue>,
}
impl Table {
    pub fn new(spec: &StructureDefinition, mut values: Vec<Vec<RawValue>>) -> Self {
        let columns: HashMap<_, _> = spec.iter().map(|c| (c.name.to_string(), c.index)).collect();
        let linearized_values: Vec<_> = values.iter_mut().flat_map(|v| v.drain(..)).collect();

        Self {
            columns,
            records: linearized_values
        }
    }
}

pub struct Record<'a> {
    table: &'a Table,
    values: &'a [RawValue]
}

impl Record<'_> {
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
    pub fn get<I>(&self, index: I) -> Option<&<I as SliceIndex<[RawValue]>>::Output>
        where I : SliceIndex<[RawValue]>
    {
        self.values.get(index)
    }
}

impl<'a, I> Index<I> for Record<'a> where [RawValue] : Index<I> {
    type Output = <[RawValue] as Index<I>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.values[index]
    }
}

impl<'a, S> Index<S> for Record<'a> where String : Borrow<S>, S: Hash + Eq  {
    type Output = RawValue;

    fn index(&self, index: S) -> &Self::Output {
        let column_index = self.table.columns[&index];
        &self.values[column_index]
    }
}