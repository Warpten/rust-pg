use std::{mem::ManuallyDrop, sync::{Arc, Mutex}};

use gpu_allocator::{vulkan::{Allocator, AllocatorCreateDesc}, AllocationSizes, AllocatorDebugSettings};

use crate::{traits::{BorrowHandle, Handle}, Framebuffer};

use super::{Queue, Context, PhysicalDevice};

/// A logical Vulkan device.
pub struct LogicalDevice {
    handle : ash::Device,
    context : Arc<Context>,
    physical_device : PhysicalDevice,
    allocator : ManuallyDrop<Arc<Mutex<Allocator>>>,

    pub queues : Vec<Queue>,
}

impl LogicalDevice {
    pub fn context(&self) -> &Arc<Context> { &self.context }
    pub fn physical_device(&self) -> &PhysicalDevice { &self.physical_device }
    pub fn allocator(&self) -> &Arc<Mutex<Allocator>> { &self.allocator }

    pub fn new(context : Arc<Context>, device : ash::Device, physical_device : PhysicalDevice, queues : Vec<Queue>) -> Self {
        let allocator = Allocator::new(&AllocatorCreateDesc{
            instance: context.handle().clone(),
            device: device.clone(),
            physical_device: physical_device.handle().clone(),

            // TODO: All these may need tweaking and fixing
            debug_settings: AllocatorDebugSettings::default(),
            allocation_sizes : AllocationSizes::default(),
            buffer_device_address: false,
        }).expect("Error creating an allocator");

        Self {
            handle : device,
            physical_device,
            context,
            queues,
            allocator : ManuallyDrop::new(Arc::new(Mutex::new(allocator)))
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
        for (i, memory_type) in self.physical_device().memory_properties().memory_types.iter().enumerate() {
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
            ManuallyDrop::drop(&mut self.allocator);

            self.handle.destroy_device(None);
        }
    }
}