use super::{Instance, PhysicalDevice};

pub struct Queue {
    pub handle : ash::vk::Queue,
    pub index : usize
}

pub struct VirtualDevice<'device, 'instance : 'device> {
    pub instance : &'instance Instance,
    pub handle : ash::Device,
    pub physical_device : &'device PhysicalDevice<'instance>,
    pub queues : Vec<Queue>
}

impl Drop for VirtualDevice<'_, '_> {
    fn drop(&mut self) {
        unsafe {
            self.handle.destroy_device(None);
        }
    }
}