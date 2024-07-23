use tokio::runtime::{Builder, Runtime};

pub struct AsyncTaskManager {
    runtime : Runtime,
}
impl AsyncTaskManager {
    pub fn new() -> Self {
        Self {
            runtime : Builder::new_multi_thread().enable_all().build().unwrap()
        }
    }
}