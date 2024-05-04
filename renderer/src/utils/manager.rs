use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc};

type Factory<T, U> = fn(&Rc<Manager<T, U>>, &'static str, usize, &U) -> T;

#[derive(Debug)]
pub struct Manager<T, U> {
    factory   : Factory<T, U>,
    entries   : RefCell<Vec<Rc<T>>>,
    index_map : HashMap<&'static str, usize>,

    _marker : PhantomData<U>,
}

impl<T, U> Manager<T, U> {
    pub fn new(factory : Factory<T, U>) -> Rc<Manager<T, U>> {
        let this = Self {
            factory,
            entries : RefCell::new(vec![]),
            index_map : HashMap::new(),

            _marker : PhantomData::default()
        };

        Rc::new(this)
    }

    pub fn len(&self) -> usize { self.entries.borrow().len() }

    pub fn for_each<Callback>(&self, callback : Callback)
        where Callback : FnMut(&Rc<T>)
    {
        todo!();
    }

    pub fn find_by_id(&self, id : usize) -> Option<Rc<T>> {
        todo!()
    }

    pub fn find(&self, name : &'static str) -> Option<Rc<T>> {
        match self.index_map.get(name) {
            Some(&index) => {
                let instances = self.entries.borrow();
                Some(Rc::clone(&instances[index]))
            },
            None => None
        }
    }

    pub fn register(self : &mut Rc<Manager<T, U>>, name : &'static str, extra : &U) -> Rc<T> {
        match self.index_map.get(name) {
            Some(&index) => {
                let instances = self.entries.borrow();
                Rc::clone(&instances[index])
            },
            None => {
                let mut instances = self.entries.borrow_mut();
                let index = instances.len();

                let instance = (self.factory)(self, name, index, extra);

                instances.push(Rc::new(instance));
                Rc::clone(&instances[index])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{borrow::{Borrow, BorrowMut}, rc::Weak};

    use super::*;

    #[derive(Debug)]
    struct Extra;
    
    #[derive(Debug)]
    struct Entry {
        pub value : i32,
        pub owner : Weak<Manager<Entry, Extra>>,
    }

    impl Entry {
        pub fn new(owner : &Rc<Manager<Entry, Extra>>) -> Entry {
            Self { value : 0, owner : Rc::downgrade(owner) }
        }
    }

    #[test]
    pub fn test() {
        let extra = Extra { };
        let mut manager = Manager::new(|this, _, _, _| Entry::new(this));
        let mut entry = manager.register("entry", &extra);

        println!("Entry parent: {:?}", manager);
    }
}