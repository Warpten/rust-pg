use std::marker::PhantomData;

pub struct Event<T> {
    _marker : PhantomData<T>,
}