use std::sync::Arc;

use super::{Instance, PhysicalDevice, QueueFamily};

pub struct Queue {
    pub handle : ash::vk::Queue,
    pub index : u32,
    pub family : QueueFamily,
}

/// A logical Vulkan device.
pub struct LogicalDevice {
    pub instance : Arc<Instance>,
    pub handle : ash::Device,
    pub physical_device : Arc<PhysicalDevice>,
    pub queues : Vec<Queue>
}

impl Drop for LogicalDevice {
    fn drop(&mut self) {
        unsafe {
            self.handle.destroy_device(None);
        }
    }
}