use tokio::{runtime::{Builder, Runtime}, sync::oneshot};
use tokio::sync::oneshot::error::TryRecvError;

/// An asynchronous task manager.
///
/// # Description
///
/// This structure is essentially a wrapper around a tokio [`Runtime`]. It allows consumer code to
/// start asynchronous tasks and obtain an handle to the eventual result of said tasks.
///
/// There is, for the moment, no concept of cancellation or timeouts.
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
    /// Attempts to read the result of this asynchronous operation and feeds it to the given lambda.
    ///
    /// # Arguments
    ///
    /// * `handler` - A lambda that will process the value.
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

    /// Attempts to return the value of this asynchronous operation.
    ///
    /// This method cannot fail spuriously.
    fn poll(&mut self) -> Result<T, TryRecvError> {
        self.rx.try_recv()
    }
}

/// An asynchronous value holder.
pub enum AsyncValue<T> {
    /// This object currently holds no value.
    None,
    /// The value of this object is currently being calculated by a task associated with the given handle.
    Pending(AsyncOneshotTaskHandle<T>),
    /// This object currently holds the given value.
    Value(T)
}
impl<T> AsyncValue<T> {
    pub fn try_poll(&mut self, handler : impl FnOnce(T)) {
        match self {
            AsyncValue::Pending(handle) => handle.try_poll(handler),
            _ => ()
        }
    }

    /// Updates this asynchronous value.
    ///
    /// # Description
    ///
    /// If this is an instance of [`AsyncValue::Pending`], the underlying handle is polled
    /// * If a value is found, this instance is updated to become [`AsyncValue::Value`].
    /// * If a value is not found because the operation was aborted or failed to complete, this instance
    ///   is updated to become [`AsyncValue::None`]
    /// * If a value is not found because the operation has not yet completed, this instance is
    ///   untouched so that subsequent calls get another chance.
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

    pub fn is_pending(&self) -> bool {
        if let AsyncValue::Pending(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_none(&self) -> bool {
        if let AsyncValue::None = self {
            true
        } else {
            false
        }
    }


    pub fn is_value(&self) -> bool {
        if let AsyncValue::Value(_) = self {
            true
        } else {
            false
        }
    }
}