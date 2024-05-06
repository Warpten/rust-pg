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
        self.entries.borrow().iter().for_each(callback)
    }

    pub fn find_by_id(&self, id : usize) -> Option<Rc<T>> {
        self.entries.borrow().get(id).map(Rc::clone)
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
