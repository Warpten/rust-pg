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

    /// Creates a new framebuffer
    /// 
    /// # Arguments
    /// 
    /// * `extent` - 
    /// * `views` - A slice of image views used to create this framebuffer.
    /// * `layers` - 
    pub fn create_framebuffer(self : Arc<Self>, extent : ash::vk::Extent2D, views : Vec<ash::vk::ImageView>, layers : u32) -> Framebuffer {
        return Framebuffer::new(extent, views, layers, self)
    }

    pub fn find_memory_type(&self, memory_type_bits : u32, flags : ash::vk::MemoryPropertyFlags) -> u32 {
        for (i, memory_type) in self.physical_device.memory_properties.memory_types.iter().enumerate() {
            if (memory_type_bits & (1 << i)) != 0 && (memory_type.property_flags & flags) == flags {
                return i as _;
            }
        }

        panic!("No memory type found matching the requirements")
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