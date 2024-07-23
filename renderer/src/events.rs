use std::future::{Future, IntoFuture};
use egui::ahash::HashMap;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

pub struct EventManager<T> {
    runtime : Runtime,
    oneshots : Vec<Box<dyn Event<T>>>,
}
impl<T> EventManager<T> {
    fn publish<E : Event<T>>(&mut self, event : E)  {
        event.spawn(&self.runtime);
        self.oneshots.push(event);
    }

    pub fn poll(&mut self) {
        self.oneshots.retain_mut(|evt| evt.poll() );
    }
}

trait Event<T> {
    fn poll(&mut self) -> bool;

    fn spawn(&self, runtime : &Runtime);
}

pub struct AsyncEvent<T, F> {
    rx : oneshot::Receiver<T>,
    handler : F,
}
impl<T, F> AsyncEvent<T, F> where F : FnOnce(T)
{
    pub fn new<F>(supplier: impl FnOnce() -> T, handler: F, mgr: &mut EventManager<T>) -> Self
        where F: FnOnce(T)
    {
        let (tx, mut rx) = oneshot::channel();

        let slf = Self { rx, handler };
        slf.spawn(&mgr.runtime, tx, supplier);
    }
}

impl<T, F> Event<T> for AsyncEvent<T, F> where F : FnOnce(T) {
    fn spawn(&self, runtime: &Runtime) {
        runtime.spawn(async move {
            self.
        });
    }
    
    fn poll(&mut self) -> bool {
        match self.rx.try_recv() {
            Ok(value) => {
                (self.handler)(value);
                false // Value received; don't retain
            },
            Err(oneshot::TryRecvError::Empty) => true, // Still waiting, retain
            Err(oneshot::TryRecvError::Closed) => false, // Closed, don't retain (this is a last ditch effort)
        }
    }
}