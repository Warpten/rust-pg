use tokio::{runtime::{Builder, Runtime}, sync::oneshot};

pub struct AsyncTaskManager {
    runtime : Runtime,
}
impl AsyncTaskManager {
    pub fn new() -> Self {
        Self {
            runtime : Builder::new_multi_thread().enable_all().build().unwrap()
        }
    }

    /// Starts an asynchronous task over the execution of the provided `supplier`.
    /// 
    /// # Arguments
    /// 
    /// * `supplier` - A lambda providing the value to publish.
    /// * `handler` - A lambda that will be called whenever [`AsyncOneshotTaskHandle::poll()`] reads a
    ///               value from the async operation.
    /// 
    /// # Returns
    /// 
    /// A handle that can be [polled](AsyncOneshotTaskHandle::poll) for a value.
    pub fn oneshot<T, S, H>(&mut self, supplier : S, handler : H) -> AsyncOneshotTaskHandle<T>
        where S : 'static + Send + FnMut() -> T, T : Send,
              H : 'static + FnMut(T)
    {
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            _ = tx.send(supplier());
        });

        AsyncOneshotTaskHandle { rx, handler: Box::new(handler) }
    }

    pub fn oneshot_maybe<T, S, H>(&mut self, supplier : S, handler : H) -> AsyncOneshotTaskHandle<T>
        where S : 'static + Send + FnMut() -> Option<T>, T : Send,
              H : 'static + FnMut(T)
    {
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            match supplier() {
                Some(value) => _ = tx.send(value),
                None => (),
            };
        });

        AsyncOneshotTaskHandle { rx, handler: Box::new(handler) }
    }
}

pub struct AsyncOneshotTaskHandle<T> {
    rx : oneshot::Receiver<T>,
    handler : Box<dyn FnMut(T)>,
}
impl<T> AsyncOneshotTaskHandle<T> {
    pub fn poll(&mut self) {
        match self.rx.try_recv() {
            Ok(value) => (self.handler)(value),
            Err(oneshot::error::TryRecvError::Empty) => { },
            Err(oneshot::error::TryRecvError::Closed) => { },
        };
    }
}

pub enum AsyncValue<T> {
    None,
    Pending(AsyncOneshotTaskHandle<T>),
    Value(T)
}
impl<T> AsyncValue<T> {
    pub fn try_poll(&mut self) {
        match self {
            AsyncValue::Pending(mut handle) => handle.poll(),
            _ => ()
        }
    }
}