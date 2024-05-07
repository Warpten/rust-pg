use std::sync::Arc;

use crate::{traits::BorrowHandle, Framebuffer};

use super::{Queue, Instance, PhysicalDevice};

/// A logical Vulkan device.
pub struct LogicalDevice {
    handle : ash::Device,
    pub instance : Arc<Instance>,
    pub physical_device : PhysicalDevice,
    pub queues : Vec<Queue>
}

impl LogicalDevice {
    pub fn new(instance : Arc<Instance>, device : ash::Device, physical_device : PhysicalDevice, queues : Vec<Queue>) -> Self {
        Self {
            handle : device,
            physical_device,
            instance,
            queues
        }
    }

    pub fn create_framebuffer(&self, extent : ash::vk::Extent2D, views : Vec<ash::vk::ImageView>, layers : u32) -> Arc<Framebuffer> {
        return Framebuffer::new(extent, views, layers, self)
    }
}

impl BorrowHandle for LogicalDevice {
    type Target = ash::Device;

    fn handle(&self) -> &ash::Device { &self.handle }
}

impl Drop for LogicalDevice {
    fn drop(&mut self) {
        unsafe {
            self.handle.destroy_device(None);
        }
    }
}