use std::{collections::HashMap, marker::PhantomData, slice::{Iter, IterMut}};

type Factory<T> = fn(&'static str, usize) -> T;

#[derive(Debug)]
pub struct Manager<T : Identifiable> {
    factory   : Factory<T>,
    entries   : Vec<T>,
    index_map : HashMap<&'static str, usize>,
}

pub trait Identifiable {
    fn name(&self) -> &'static str;
    fn id(&self) -> usize;
}

/// An identifier type.
/// 
/// This sum type can be either a [`Identifier::Numeric`] or a [`Identifier::Named`]. The former corresponds
/// to the index of the identified object in its pool, and the latter to its name. Both values should be unique.
pub enum Identifier<T : Identifiable> {
    Numeric(usize, PhantomData<T>),
    Named(&'static str, PhantomData<T>)
}

impl<T : Identifiable> Clone for Identifier<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Numeric(arg0, arg1) => Self::Numeric(arg0.clone(), arg1.clone()),
            Self::Named(arg0, arg1) => Self::Named(arg0.clone(), arg1.clone()),
        }
    }
}

impl<T : Identifiable> Identifier<T> {
    pub fn get_mut(self, owner : &mut Manager<T>) -> &mut T {
        owner.find_mut(self).unwrap()
    }

    pub fn get(self, owner : &Manager<T>) -> &T {
        owner.find(self).unwrap()
    }
}

impl<T : Identifiable> Into<Identifier<T>> for usize {
    fn into(self) -> Identifier<T> {
        Identifier::<T>::Numeric(self, PhantomData::default())
    }
}

impl<T : Identifiable> Into<Identifier<T>> for &'static str {
    fn into(self) -> Identifier<T> {
        Identifier::<T>::Named(self, PhantomData::default())
    }
}

impl<T : Identifiable> Manager<T> {
    /// Creates a new object manager.
    /// 
    /// # Arguments
    /// 
    /// * `factory` - A callable returning a new instance of T.
    pub fn new(factory : Factory<T>) -> Manager<T> {
        Self {
            factory,
            entries : vec![],
            index_map : HashMap::new(),
        }
    }

    /// Returns the amount of entries in this manager.
    pub fn len(&self) -> usize { self.entries.len() }

    pub fn iter(&self) -> Iter<'_, T> {
        self.entries.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.entries.iter_mut()
    }

    /// Finds an object in the pool, given its identifier.
    /// 
    /// # Arguments
    /// 
    /// * `identifier` - An [`Identifier`]. See the associated documentation for an explanation on the values allowed here.
    pub fn find_mut(&mut self, identifier : Identifier<T>) -> Option<&mut T> {
        match identifier {
            Identifier::<T>::Named(name, _) => {
                self.index_map.get(name).and_then(|&index| self.entries.get_mut(index))
            },
            Identifier::<T>::Numeric(index, _) => {
                self.entries.get_mut(index)
            }
        }
    }
    
    /// Finds an object in the pool, given its identifier.
    /// 
    /// # Arguments
    /// 
    /// * `identifier` - An [`Identifier`]. See the associated documentation for an explanation on the values allowed here.
    pub fn find(&self, identifier : Identifier<T>) -> Option<&T> {
        match identifier {
            Identifier::<T>::Named(name, _) => {
                self.index_map.get(name).and_then(|&index| self.entries.get(index))
            },
            Identifier::<T>::Numeric(index, _) => {
                self.entries.get(index)
            }
        }
    }

    /// Registers a new instance of the managed type and returns it.
    /// 
    /// # Arguments
    /// 
    /// * `name` - An uniquely identifying name for the new instance.
    pub fn register(&mut self, name : &'static str) -> &mut T {
        // self.register_deferred(name, self.factory) // Can't I just do this?
        let index = self.entries.len();
        let instance = (self.factory)(name, index);

        self.index_map.insert(name, index);
        self.entries.push(instance);

        &mut self.entries[index]
    }

    /// Registers a new instance of the managed type and returns it, using the provided factory instead of the
    /// default one.
    /// 
    /// # Arguments
    /// 
    /// * `name` - An uniquely identifying name for the new instance.
    /// * `factory` - A callable returning a new instance of the managed type.
    pub fn register_deferred<F>(&mut self, name : &'static str, factory : F) -> &mut T
        where F : Fn(&'static str, usize) -> T
    {
        let index = self.entries.len();
        let instance = factory(name, index);

        self.index_map.insert(name, index);
        self.entries.push(instance);

        &mut self.entries[index]
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.index_map.clear();
    }
}
