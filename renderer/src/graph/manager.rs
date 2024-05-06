use std::{collections::HashMap, slice::{IterMut, Iter}};

#[derive(Debug)]
pub struct Manager<T : Identifiable> {
    entries   : Vec<T>,
    index_map : HashMap<&'static str, usize>,
}

pub trait Identifiable {
    fn name(&self) -> &'static str;
}

impl<T : Identifiable> Manager<T> {
    pub fn new() -> Manager<T> {
        Self {
            entries : vec![],
            index_map : HashMap::new(),
        }
    }

    pub fn len(&self) -> usize { self.entries.len() }

    pub fn iter(&self) -> Iter<'_, T> {
        self.entries.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.entries.iter_mut()
    }

    pub fn find(&self, name : &'static str) -> Option<&T> {
        match self.index_map.get(name) {
            Some(&index) => {
                Some(&self.entries[index])
            },
            None => None
        }
    }

    pub fn register(&mut self, instance : T) {
        self.entries.push(instance);
        self.index_map.insert(instance.name(), self.entries.len() - 1);
    }
}
