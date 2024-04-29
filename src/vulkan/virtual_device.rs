use super::{Instance, PhysicalDevice};

pub struct Queue {
    pub handle : ash::vk::Queue,
    pub index : usize
}

pub struct VirtualDevice<'device, 'instance : 'device> {
    pub instance : &'instance Instance,
    pub physical_device : &'device PhysicalDevice<'instance>,
    pub queues : Vec<Queue>
}
