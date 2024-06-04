use ash::vk;

pub trait Handle<T : vk::Handle> {
    fn handle(&self) -> T;
}

/// Implements the [`Handle`] trait over a slice.
/// 
/// This trait should be blanket implemented. Implement [`Handle<T>`] on your individual types instead.
pub trait Handles<T> {
    fn handles(&self) -> Vec<T>;
}

impl<T, U> Handles<T> for &[U] where U : Handle<T>, T : vk::Handle {
    fn handles(&self) -> Vec<T> {
        let mut store = Vec::with_capacity(self.len());
        for i in 0..self.len() { store.push(self[i].handle()); }
        store
    }
}
