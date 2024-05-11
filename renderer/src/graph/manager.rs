use std::collections::HashMap;

pub trait Identifiable {
    type Key : Into<Identifier>;

    fn name(&self) -> &'static str;
    fn id(&self) -> Self::Key;
}

pub struct Manager<T : Identifiable> {
    entries : Vec<T>,
    name_map : HashMap<&'static str, usize>,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[allow(dead_code)]
pub enum Identifier {
    Numeric(usize),
    Named(&'static str),
    None,
}

impl<T : Identifiable> Manager<T> {
    pub fn find<K>(&self, key : K) -> Option<&T>
        where K : From<T::Key>, Identifier : From<K>
    {
        let identifier = Identifier::from(key);

        match identifier {
            Identifier::Numeric(index) => self.entries.get(index),
            Identifier::Named(name) => self.name_map.get(name).and_then(|&index| self.entries.get(index)),
            Identifier::None => None,
        }
    }

    pub(in super) fn find_mut<K>(&mut self, key : K) -> Option<&mut T>
        where K : From<T::Key>, Identifier : From<K>
    {
        match Identifier::from(key) {
            Identifier::Numeric(index) => self.entries.get_mut(index),
            Identifier::Named(name) => self.name_map.get(name).and_then(|&index| self.entries.get_mut(index)),
            Identifier::None => None,
        }
    }

    pub fn len(&self) -> usize { self.entries.len() }

    pub fn iter(&self) -> std::slice::Iter<T> {
        self.entries.iter()
    }

    pub(in super) fn register<F>(&mut self, mut entry : T, id_setter : F) -> &T
        where F : Fn(&mut T, usize)
    {
        match self.name_map.get(entry.name()) {
            Some(_) => panic!("An object with this name already exists"),
            None => {
                let index = self.entries.len();
                id_setter(&mut entry, index);

                self.name_map.insert(entry.name(), index);
                self.entries.push(entry);

                &self.entries[index]
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