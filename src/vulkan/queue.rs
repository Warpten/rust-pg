use std::{hash::Hash, sync::Arc};

use super::{CommandPool, LogicalDevice, PhysicalDevice, Surface};

#[derive(Clone, Copy)]
pub struct QueueFamily {
    pub index : usize,
    pub properties : ash::vk::QueueFamilyProperties,
}

// Have to implement these manually because ash doesn't derive Eq, PartialEq, and Hash for QFPs.
impl PartialEq for QueueFamily {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
            && self.properties.queue_flags == other.properties.queue_flags
            && self.properties.queue_count == other.properties.queue_count
            && self.properties.timestamp_valid_bits == other.properties.timestamp_valid_bits
            && self.properties.min_image_transfer_granularity == other.properties.min_image_transfer_granularity
    }
}

impl Eq for QueueFamily { }

impl Hash for QueueFamily {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.properties.queue_flags.hash(state);
        self.properties.queue_count.hash(state);
        self.properties.timestamp_valid_bits.hash(state);
        self.properties.min_image_transfer_granularity.hash(state);
    }
}

impl QueueFamily {
    /// Returns true if this queue family can present to a given surface for a physical device.
    ///
    /// # Arguments
    /// 
    /// * `surface` - The [`Surface`] on which to present.
    /// * `device` - The [`PhysicalDevice`] for which to present.
    /// 
    /// # Panics
    ///
    /// * Panics if [`vkGetPhysicalDeviceSurfaceSupportKHR`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkGetPhysicalDeviceSurfaceSupportKHR.html) fails.
    /// * Panics if the provided [`Surface`] has been dropped before this call happens.
    pub fn can_present(&self, surface : Arc<Surface>, device : Arc<PhysicalDevice>) -> bool {
        unsafe {
            surface.loader.get_physical_device_surface_support(
                device.handle,
                self.index as u32,
                surface.handle
            ).expect("Failed to get physical device surface support")
        }
    }

    ///
    /// Creates a command pool.
    /// 
    /// # Arguments
    /// 
    /// * `device` - The device for which the command pool will be created.
    /// 
    pub fn create_command_pool(
        &self,
        device : Arc<LogicalDevice>
    ) -> Arc<CommandPool> {
        let command_pool = {
            let command_pool_create_info = ash::vk::CommandPoolCreateInfo::default()
                .flags(ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(self.index as u32);
            unsafe {
                device.handle
                    .create_command_pool(&command_pool_create_info, None)
                    .expect("Failed to create command pool")
            }
        };

        Arc::new(CommandPool { handle : command_pool, device })
    }
}