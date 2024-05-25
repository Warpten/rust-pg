use ash::vk;

pub trait Handle<T : vk::Handle> {
    fn handle(&self) -> T;
}
