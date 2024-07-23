use tokio::{runtime::{Builder, Runtime}, sync::oneshot};
use tokio::sync::oneshot::error::TryRecvError;

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
    /// 
    /// # Returns
    /// 
    /// A handle that can be [polled](AsyncOneshotTaskHandle::poll) for a value.
    pub fn oneshot<T, S>(&mut self, supplier : S) -> AsyncOneshotTaskHandle<T>
        where S : 'static + Send + FnOnce() -> T,
              T : 'static + Send
    {
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            _ = tx.send(supplier());
        });

        AsyncOneshotTaskHandle { rx }
    }

    /// Starts an asynchronous task over the execution of the provided `supplier`.
    ///
    /// # Arguments
    ///
    /// * `supplier` - A lambda providing an optional value to publish. If the optional is [None], no
    ///                value is sent.
    ///
    /// # Returns
    ///
    /// A handle that can be [polled](AsyncOneshotTaskHandle::poll) for a value.
    pub fn oneshot_maybe<T, S>(&mut self, supplier : S) -> AsyncOneshotTaskHandle<T>
        where S : 'static + Send + FnOnce() -> Option<T>,
              T : 'static + Send
    {
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            match supplier() {
                Some(value) => _ = tx.send(value),
                None => (),
            };
        });

        AsyncOneshotTaskHandle { rx }
    }
}

pub struct AsyncOneshotTaskHandle<T> {
    rx : oneshot::Receiver<T>,
}
impl<T> AsyncOneshotTaskHandle<T> {
    pub fn try_poll(&mut self, handler : impl FnOnce(T)) {
        match self.rx.try_recv() {
            Ok(value) => handler(value),
            Err(TryRecvError::Empty) => { },
            Err(TryRecvError::Closed) => { },
        };
    }

    fn poll_value(&mut self) -> Option<T> {
        match self.rx.try_recv() {
            Ok(value) => Some(value),
            Err(_) => None,
        }
    }

    fn poll(&mut self) -> Result<T, TryRecvError> {
        self.rx.try_recv()
    }
}

pub enum AsyncValue<T> {
    None,
    Pending(AsyncOneshotTaskHandle<T>),
    Value(T)
}
impl<T> AsyncValue<T> {
    pub fn try_poll(&mut self, handler : impl FnOnce(T)) {
        match self {
            AsyncValue::Pending(handle) => handle.try_poll(handler),
            _ => ()
        }
    }

    /// Updates this asynchronous value
    pub fn try_update(&mut self) {
        match self {
            AsyncValue::Pending(value) => {
                match value.poll() {
                    Ok(value) => *self = AsyncValue::Value(value),
                    Err(TryRecvError::Closed) => *self = AsyncValue::None,
                    Err(TryRecvError::Empty) => { /* do nothing, still waiting */ }
                }
            },
            _ => (),
        };
    }
}