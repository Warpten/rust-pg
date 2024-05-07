use std::{collections::HashMap, hash::Hash, marker::PhantomData, thread::Builder};

struct Pool<T : Identifiable> {
    entries : Vec<T>,
    names : HashMap<T::Identifier, usize>,
}

struct Opaque<T> {
    index : usize,
    _marker : PhantomData<T>,
}

impl<T : Identifiable> Pool<T> {
    pub fn register(&mut self, obj : T) -> Opaque<T> {
        match self.names.get(&obj.name()) {
            Some(index) => panic!("An object with that identifier already exists"),
            None => {
                let index = self.entries.len();
                self.names.insert(obj.name(), index);
                self.entries.push(obj);

                Opaque { index, _marker : PhantomData::default() }
            }
        }
    }

    pub fn find(&self, id : T::Identifier) -> Option<Opaque<T>> {
        match self.names.get(&id) {
            Some(&index) => Some(Opaque { index, _marker : PhantomData::default() }),
            None => None
        }
    }

    pub fn len(&self) -> usize { self.entries.len() }
}

trait Identifiable {
    type Identifier : Eq + Ord + PartialOrd + PartialEq + Hash;

    fn name(&self) -> Self::Identifier;
}

struct Pass {
    #[cfg(debug_assertions)]
    registered : bool,
}

impl Pass {
}

#[cfg(debug_assertions)]
impl Drop for Pass {
    fn drop(&mut self) {
        assert!(self.registered, "This pass did not get registered; this is probably a bug")
    }
}

impl Default for Pass {
    fn default() -> Self {
        Self { registered : false  }
    }
}