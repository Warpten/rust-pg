use std::{hash::Hash, sync::Arc};

use ash::vk;
use bitmask_enum::bitmask;

use crate::make_handle;
use crate::{traits::handle::Handle};
use crate::vk::command_pool::CommandPool;
use crate::vk::logical_device::LogicalDevice;
use crate::vk::physical_device::PhysicalDevice;
use crate::vk::surface::Surface;

/// A logical queue associated with a logical device.
pub struct Queue {
    handle : vk::Queue,
    index : u32,
    family : QueueFamily,
}

#[bitmask(u8)]
pub enum QueueAffinity {
    Compute,
    Graphics,
    Transfer,
}

impl Queue {
    pub fn new(family : &QueueFamily, index : u32, device : &ash::Device) -> Self {
        Self {
            index,
            family : *family,
            handle : unsafe {
                device.get_device_queue(family.index, index)
            }
        }
    }

    pub fn affinity(&self) -> QueueAffinity {
        let mut affinity = QueueAffinity::none();
        if self.family.is_compute() {
            affinity = affinity.or(QueueAffinity::Compute);
        }
        if self.family.is_graphics() {
            affinity = affinity.or(QueueAffinity::Graphics)
        };
        if self.family.is_transfer() {
            affinity = affinity.or(QueueAffinity::Transfer)
        };
        affinity
    }

    #[inline] pub fn index(&self) -> u32 { self.index }
    #[inline] pub fn family_index(&self) -> u32 { self.family.index() }
    #[inline] pub fn is_graphics(&self) -> bool { self.family.is_graphics() }
    #[inline] pub fn is_compute(&self) -> bool { self.family.is_compute() }
    #[inline] pub fn is_transfer(&self) -> bool { self.family.is_transfer() }
    #[inline] pub fn family(&self) -> &QueueFamily { &self.family }
}

make_handle! { Queue, vk::Queue }

/// A queue family.
/// 
/// This structure associates, for a particular physical device, a queue family's properties with its index.
///
/// # Properties
/// 
/// * `queueFlags` - Indicates capabilities of the queues in this queue family.
/// * `queueCount` - Amount of queues in this queue family. All families **must** support at least one queue.
/// * `timestampValidBits` - This is the amount of meaningful bits in the timestamp written via
///   [`vkCmdWriteTimestamp2`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCmdWriteTimestamp2.html)
///   or [`vkCmdWriteTimestamp`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCmdWriteTimestamp.html).
///   The valid range for the count is 36 to 64 bits, or a value of 0, indicating no support for timestamps. Bits outside
///   the valid range are guaranteed to be zeros.
/// * `minImageTransferGranularity` is the minimum granularity supported for image transfer operations on the queues in this queue family.
#[derive(Clone, Copy)]
pub struct QueueFamily {
    /// The index of this queue family.
    index : u32,
    /// An object describing properties of this queue family.
    properties : vk::QueueFamilyProperties,
}

impl QueueFamily {
    pub fn new(index : u32, properties : vk::QueueFamilyProperties) -> Self {
        Self { index, properties }
    }

    #[inline] pub fn index(&self) -> u32 { self.index }

    /// Checks if this queue family supports graphics operations.
    #[inline] pub fn is_graphics(&self) -> bool { self.properties.queue_flags.contains(vk::QueueFlags::GRAPHICS) }
    
    /// Checks if this queue family supports compute operations.
    #[inline] pub fn is_compute(&self) -> bool { self.properties.queue_flags.contains(vk::QueueFlags::COMPUTE) }

    /// Checks if this queue family supports transfer operations.
    #[inline] pub fn is_transfer(&self) -> bool { self.properties.queue_flags.contains(vk::QueueFlags::TRANSFER) || self.is_compute() || self.is_graphics() }

    #[inline] pub fn min_image_transfer_granularity(&self) -> vk::Extent3D {
        self.properties.min_image_transfer_granularity
    }

    #[inline] pub fn count(&self) -> u32 {
        self.properties.queue_count
    }


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
    pub fn can_present(&self, surface : &Arc<Surface>, device : &PhysicalDevice) -> bool {
        unsafe {
            surface.loader.get_physical_device_surface_support(
                device.handle(),
                self.index,
                surface.handle()
            ).expect("Failed to get physical device surface support")
        }
    }

    /// Creates a command pool.
    /// 
    /// # Arguments
    /// 
    /// * `device` - The device for which the command pool will be created.
    /// 
    /// # Panics
    /// 
    /// * Panics if [`vkCreateCommandPool`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCreateCommandPool.html) fails.
    pub fn create_command_pool(&self, device : &Arc<LogicalDevice>) -> Arc<CommandPool> {
        Arc::new(CommandPool::create(&self, device))
    }
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