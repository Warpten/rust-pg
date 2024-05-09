use std::{collections::HashMap, marker::PhantomData};

pub trait Identifiable {
    type Key;

    fn name(&self) -> &'static str;
    fn id(&self) -> Self::Key;
}

pub struct Manager<T : Identifiable> {
    entries : Vec<T>,
    name_map : HashMap<&'static str, usize>,
}

pub enum Identifier<T> {
    Numeric(usize, PhantomData<T>),
    Named(&'static str, PhantomData<T>),
}

impl<T : Identifiable> Manager<T> {
    pub fn find(&self, identifier : Identifier<T::Key>) -> Option<&T> {
        match identifier {
            Identifier::Numeric(index, _) => self.entries.get(index),
            Identifier::Named(name, _) => self.name_map.get(name).and_then(|&index| self.entries.get(index))
        }
    }

    pub fn len(&self) -> usize { self.entries.len() }

    pub fn iter(&self) -> std::slice::Iter<T> {
        self.entries.iter()
    }

    pub(in super) fn register<F>(&mut self, mut entry : T, id_setter : F) -> <T as Identifiable>::Key
        where F : Fn(&mut T, usize)
    {
        match self.name_map.get(entry.name()) {
            Some(_) => panic!("An object with this name already exists"),
            None => {
                let index = self.entries.len();
                id_setter(&mut entry, index);

                self.name_map.insert(entry.name(), index);
                self.entries.push(entry);

                self.entries[index].id()
            }
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.name_map.clear();
    }
}

impl<T : Identifiable> Default for Manager<T> {
    fn default() -> Self {
        Self { entries: vec![], name_map: Default::default() }
    }
}