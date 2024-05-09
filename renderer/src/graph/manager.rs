use std::collections::HashMap;

pub trait Identifiable {
    type Key;

    fn name(&self) -> &'static str;
    fn id(&self) -> Self::Key;
}

pub struct Manager<T : Identifiable> {
    entries : Vec<T>,
    name_map : HashMap<&'static str, usize>,
}

pub enum Identifier {
    Numeric(usize),
    Named(&'static str),
}

impl<T : Identifiable> Manager<T> {
    pub fn find(&self, identifier : Identifier) -> Option<&T> {
        match identifier {
            Identifier::Numeric(index) => self.entries.get(index),
            Identifier::Named(name) => self.name_map.get(name).and_then(|&index| self.entries.get(index))
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
}

impl<T : Identifiable> Default for Manager<T> {
    fn default() -> Self {
        Self { entries: vec![], name_map: Default::default() }
    }
}