use std::{collections::HashMap, ffi::CString};

use anyhow::ensure;

use super::{Instance, Queue, VirtualDevice};

#[derive(Clone, Copy)]
pub struct QueueFamily {
    pub index : usize,
    pub properties : ash::vk::QueueFamilyProperties,
}

impl QueueFamily {
    /// Creates a new physical device.
    /// 
    /// # Arguments
    /// 
    /// * `instance` - An [`Instance`] modeling vulkan stuff. God knows.
    /// * `physical_device` - The [`PhysicalDevice`] attached to this queue family.
    /// * `queue_count` - The amount of queues to create. This can never be more than the actual number of queues in this family and will be clamped down.
    /// * `get_queue_priority` - A callable that will return a queue's priority given its index.
    /// * `extensions` - A set of device extensions to be enabled on the device.
    pub fn create_virtual_device<'instance, 'device : 'instance>(
        &self,
        instance : &'instance Instance,
        physical_device : &'device PhysicalDevice<'instance>,
        queue_count : usize,
        get_queue_priority : &dyn Fn(usize) -> f32,
        extensions : Vec<CString>,
    ) -> VirtualDevice<'device, 'instance> {
        let queues = (0..queue_count).take(self.properties.queue_count as usize).collect::<Vec<_>>();

        let queue_priorities = queues
            .iter()
            .map(|&i| get_queue_priority(i))
            .collect::<Vec<_>>();

        let queue_create_info = vec![ash::vk::DeviceQueueCreateInfo::default()
            .queue_family_index(self.index as u32)
            .queue_priorities(&queue_priorities)];

        let physical_device_features = ash::vk::PhysicalDeviceFeatures::default();

        let enabled_extension_names = extensions
            .iter()
            .map(|s| s.as_ptr())
            .collect::<Vec<_>>();

        let device_create_info = ash::vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_info)
            .enabled_features(&physical_device_features)
            .enabled_extension_names(&enabled_extension_names);

        let device = unsafe {
            instance.handle.create_device(physical_device.handle, &device_create_info, None)
                .expect("Failed to create a virtual device")
        };

        // Now, get all the queues
        let queues_objs = queues.iter().map(|&queue_index| {
            Queue {
                index : queue_index,
                handle : unsafe {
                    device.get_device_queue(self.index as u32, queue_index as u32)
                }
            }
        }).collect::<Vec<_>>();

        VirtualDevice {
            instance,
            handle : device,
            physical_device,
            queues : queues_objs
        }
    }
}

#[derive(Clone)]
pub struct PhysicalDevice<'instance> {
    pub instance : &'instance Instance,
    pub handle : ash::vk::PhysicalDevice,
    pub properties : ash::vk::PhysicalDeviceMemoryProperties,
    pub queue_families : HashMap<usize, QueueFamily>,
}

impl<'instance> PhysicalDevice<'instance> {
    pub fn get_extensions(&self) -> Vec<ash::vk::ExtensionProperties> {
        unsafe {
            self.instance.handle.enumerate_device_extension_properties(self.handle)
                .expect("Failed to enumerate device extensions")
        }
    }

    pub fn new(
        device : ash::vk::PhysicalDevice,
        instance : &'instance Instance
    ) -> Self {
        let physical_device_memory_properties = unsafe {
            instance.handle.get_physical_device_memory_properties(device)
        };

        let queue_families = unsafe {
            instance.handle.get_physical_device_queue_family_properties(device)
        }.iter().enumerate().map(|(index, properties)| {
            (index, QueueFamily { index, properties : *properties })
        }).collect::<HashMap<_, _>>();

        Self {
            handle : device,
            instance,
            properties : physical_device_memory_properties,
            queue_families
        }
    }
}