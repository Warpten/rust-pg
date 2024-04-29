use super::{Instance, PhysicalDevice};

pub struct Queue {
    pub handle : ash::vk::Queue,
    pub index : usize
}

pub struct LogicalDevice<'device, 'instance : 'device> {
    pub instance : &'instance Instance,
    pub handle : ash::Device,
    pub physical_device : &'device PhysicalDevice<'instance>,
    pub queues : Vec<Queue>
}

impl Drop for LogicalDevice<'_, '_> {
    fn drop(&mut self) {
        unsafe {
            self.handle.destroy_device(None);
        }
    }
}