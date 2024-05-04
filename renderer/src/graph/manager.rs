use std::{borrow::Borrow, cell::RefCell, collections::HashMap, rc::Rc};

type Factory<T> = fn(&Rc<Manager<T>>, &'static str, usize) -> T;

#[derive(Debug)]
pub struct Manager<T> {
    factory   : Factory<T>,
    entries   : RefCell<Vec<Rc<T>>>,
    index_map : HashMap<&'static str, usize>,
}

impl<T> Manager<T> {
    pub fn new(factory : Factory<T>) -> Rc<Manager<T>> {
        let this = Self {
            factory,
            entries : RefCell::new(vec![]),
            index_map : HashMap::new(),
        };

        Rc::new(this)
    }

    pub fn for_each<Callback>(&self, callback : Callback)
        where Callback : Fn(&T)
    {
        self.entries.borrow().iter().for_each(|item| {
            callback(Rc::borrow(item));
        });
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

    pub fn register(self : &mut Rc<Manager<T>>, name : &'static str) -> Rc<T> {
        match self.index_map.get(name) {
            Some(&index) => {
                let instances = self.entries.borrow();
                Rc::clone(&instances[index])
            },
            None => {
                let mut instances = self.entries.borrow_mut();
                let index = instances.len();

                let instance = (self.factory)(self, name, index);

                instances.push(Rc::new(instance));
                Rc::clone(&instances[index])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Weak;

    use super::*;
    
    #[derive(Debug)]
    struct Entry {
        pub value : i32,
        pub owner : Weak<Manager<Entry>>,
    }

    impl Entry {
        pub fn new(owner : &Rc<Manager<Entry>>) -> Entry {
            Self { value : 0, owner : Rc::downgrade(owner) }
        }
    }

    #[test]
    pub fn test() {
        let mut manager = Manager::new(|this, _, _| Entry::new(&this));
        let entry = manager.register("entry");

        let manager_ref = &entry.owner;

        println!("Entry parent: {:?}, value: {:?}", manager_ref.upgrade(), entry.value);
    }
}