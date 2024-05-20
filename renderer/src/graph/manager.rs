use std::collections::HashMap;
use crate::graph::resource::Identifiable;

pub struct Manager<T : Identifiable> {
    entries : Vec<T>,
    names : HashMap<&'static str, usize>,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Identifier {
    Numeric(usize),
    Named(&'static str),
}

impl<T : Identifiable> Manager<T> {
    pub(in crate) fn register<F>(&mut self, mut value : T, setter : F) -> &T
        where F : FnOnce(&mut T, usize)
    {
        match self.names.get(value.name()) {
            Some(_) => panic!("An object with this name already exists"),
            None => {
                let index = self.entries.len();
                setter(&mut value, index);

                self.names.insert(value.name(), index);
                self.entries.push(value);

                &self.entries[index]
            }
        }
    }

    pub(in crate) fn iter(&self) -> std::slice::Iter<T> {
        self.entries.iter()
    }
}

impl<T : Identifiable> Manager<T> {
    pub fn find<I>(&self, identifier : I) -> Option<&T>
        where I : Into<Identifier>
    {
        match identifier.into() {
            Identifier::Numeric(index) => self.entries.get(index),
            Identifier::Named(name) => {
                self.names.get(name)
                    .and_then(|idx| self.entries.get(*idx))
            }
        }
    }

    pub(in crate) fn find_mut<I>(&mut self, identifier : I) -> Option<&mut T>
        where  I : Into<Identifier>
    {
        match identifier.into() {
            Identifier::Numeric(index) => self.entries.get_mut(index),
            Identifier::Named(name) => {
                self.names.get(name)
                    .and_then(|idx| self.entries.get_mut(*idx))
            }
        }
    }

    pub fn clear(&mut self) {
        self.names.clear();
        self.entries.clear();
    }
}

impl<T : Identifiable> Default for Manager<T> {
    fn default() -> Self {
        Self { entries : vec![], names : HashMap::default() }
    }
}