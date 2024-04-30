use std::{collections::HashMap, ffi::CString, ops::Deref, sync::Arc};

use super::{QueueFamily, Instance, LogicalDevice, Queue};

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
                    index,
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
    /// 
    /// # Arguments
    /// 
    /// * `device` - The physical device backing this logical device.
    /// * `instance` - The global Vulkan instance.
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