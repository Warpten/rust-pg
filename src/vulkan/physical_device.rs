use std::{collections::HashMap, ffi::CString, hash::Hash, ops::Deref, sync::{Arc, Weak}};

use anyhow::ensure;

use super::{CommandPool, Instance, LogicalDevice, Queue, Surface};

#[derive(Clone, Copy)]
pub struct QueueFamily {
    pub index : usize,
    pub properties : ash::vk::QueueFamilyProperties,
}

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

#[derive(Clone)]
pub struct PhysicalDevice {
    pub instance : Arc<Instance>,
    pub handle : ash::vk::PhysicalDevice,
    pub memory_properties : ash::vk::PhysicalDeviceMemoryProperties,
    pub properties : ash::vk::PhysicalDeviceProperties,
    pub queue_families : Vec<QueueFamily>,
}

impl Deref for PhysicalDevice {
    type Target = ash::vk::PhysicalDevice;

    fn deref(&self) -> &Self::Target { &self.handle }
}

impl PhysicalDevice {
    /// Returns the extensions of this [`PhysicalDevice`].
    ///
    /// # Panics
    ///
    /// Panics if [`vkEnumerateDeviceExtensionProperties`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkEnumerateDeviceExtensionProperties.html) fails.
    pub fn get_extensions(&self) -> Vec<ash::vk::ExtensionProperties> {
        unsafe {
            self.instance.handle.enumerate_device_extension_properties(self.handle)
                .expect("Failed to enumerate device extensions")
        }
    }
    
    /// Creates a new physical device.
    /// 
    /// # Arguments
    /// 
    /// * `instance` - An [`Instance`] modeling vulkan stuff. God knows.
    /// * `physical_device` - The [`PhysicalDevice`] attached to this queue family.
    /// * `queue_families` - A vector of queue families to use for this logical device, along with the requested number of queues for each family.
    /// * `get_queue_priority` - A callable that will return a queue's priority given its index.
    /// * `extensions` - A set of device extensions to be enabled on the device.
    /// 
    /// # Panics
    /// 
    /// * Panics if [`vkCreateDevice`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCreateDevice.html) fails.
    pub fn create_logical_device<F>(
        self : Arc<PhysicalDevice>,
        instance : Arc<Instance>,
        queue_families : Vec<(u32, &QueueFamily)>,
        get_queue_priority : F,
        extensions : Vec<CString>,
    ) -> Arc<LogicalDevice>
        where F : Fn(u32, QueueFamily) -> f32
    {
        // If we have the same queue family twice (for example, a graphics queue family and a present-able queue family are the same)
        // deduplicate them.
        // That implies we must sum up the queue counts - we'll limit them later
        let queue_families = {
            let mut work_buffer = HashMap::<QueueFamily, u32>::new();
            for (count, &family) in queue_families {
                work_buffer.entry(family).and_modify(|v| { *v += count; }).or_insert(count);
            }

            work_buffer.iter().map(|e| (*e.1, *e.0)).collect::<Vec<_>>()
        };

        let queue_create_infos = queue_families.iter().map(|&(count, family)| {
            let queues = (0..count).take(family.properties.queue_count as usize).collect::<Vec<_>>();

            let queue_priorities = queues
                .iter()
                .map(|&i| get_queue_priority(i, family))
                .collect::<Vec<_>>();

            ash::vk::DeviceQueueCreateInfo::default()
                .queue_family_index(family.index as u32)
                .queue_priorities(&queue_priorities)
        }).collect::<Vec<_>>();

        let physical_device_features = ash::vk::PhysicalDeviceFeatures::default();

        let enabled_extension_names = extensions
            .iter()
            .map(|s| s.as_ptr())
            .collect::<Vec<_>>();

        let device_create_info = ash::vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_features(&physical_device_features)
            .enabled_extension_names(&enabled_extension_names);

        let device = unsafe {
            instance.create_device(self.handle, &device_create_info, None)
                .expect("Failed to create a virtual device")
        };

        // Now, get all the queues
        let queues_objs = queue_families.iter().flat_map(|&(count, family)| {
            (0..count).map(|index| {
                Queue {
                    family,
                    index : index,
                    handle : unsafe { device.get_device_queue(family.index as u32, index) }
                }
            })
        }).collect::<Vec<_>>();

        Arc::new(LogicalDevice {
            instance,
            handle : device,
            physical_device : self,
            queues : queues_objs,
        })
    }

    /// Creates a new [`PhysicalDevice`].
    pub fn new(
        device : ash::vk::PhysicalDevice,
        instance : Arc<Instance>
    ) -> Arc<Self> {
        let physical_device_memory_properties = unsafe {
            instance.handle.get_physical_device_memory_properties(device)
        };

        let physical_device_properties = unsafe {
            instance.handle.get_physical_device_properties(device)
        };

        let queue_families = unsafe {
            instance.handle.get_physical_device_queue_family_properties(device)
        }.iter().enumerate().map(|(index, &properties)| {
            QueueFamily { index, properties }
        }).collect::<Vec<_>>();

        Arc::new(Self {
            handle : device,
            instance,
            memory_properties : physical_device_memory_properties,
            properties : physical_device_properties,
            queue_families
        })
    }
}