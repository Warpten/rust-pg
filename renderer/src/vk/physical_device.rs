use std::{cmp::min, ffi::CString, ops::Range, sync::{Arc, Weak}};

use ash::vk;
use crate::{traits::handle::{BorrowHandle, Handle}, vk::IndexingFeatures};
use crate::vk::{QueueFamily, Context, LogicalDevice, Queue};

#[derive(Clone)]
pub struct PhysicalDevice {
    handle : vk::PhysicalDevice,
    context : Weak<Context>,
    memory_properties : vk::PhysicalDeviceMemoryProperties,
    properties : vk::PhysicalDeviceProperties,
    pub queue_families : Vec<QueueFamily>,
}

impl Handle for PhysicalDevice {
    type Target = vk::PhysicalDevice;

    fn handle(&self) -> vk::PhysicalDevice { self.handle }
}

impl PhysicalDevice {
    #[inline] pub fn context(&self) -> &Weak<Context> { &self.context }
    #[inline] pub fn memory_properties(&self) -> &vk::PhysicalDeviceMemoryProperties { &self.memory_properties }
    #[inline] pub fn properties(&self) -> &vk::PhysicalDeviceProperties { &self.properties }

    /// Returns the extensions available on this [`PhysicalDevice`].
    ///
    /// # Panics
    ///
    /// Panics if [`vkEnumerateDeviceExtensionProperties`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkEnumerateDeviceExtensionProperties.html) fails.
    pub fn get_extensions(&self) -> Vec<vk::ExtensionProperties> {
        unsafe {
            self.context.upgrade()
                .expect("Instance released too early")
                .handle()
                .enumerate_device_extension_properties(self.handle)
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
        &self,
        instance : &Arc<Context>,
        queue_families : Vec<(u32, &QueueFamily)>,
        get_queue_priority : F,
        extensions : &Vec<CString>,
    ) -> Arc<LogicalDevice>
        where F : Fn(u32, &QueueFamily) -> f32
    {
        // Store queue priorities in a flattened buffer; each queue family will index into
        // that buffer to slice out the amount of queue families.
        let mut queue_create_infos = Vec::with_capacity(queue_families.len());
        let mut flat_queue_priorities = vec![];

        // Unfortunately has to happen in two loops because one borrow is immutable
        // and the other is mutable...
        for (count, family) in queue_families.iter() {
            for queue_index in 0..min(family.count(), *count) {
                flat_queue_priorities.push(get_queue_priority(queue_index, family));
            }
        }
        
        let mut priority_index = 0;
        for (count, family) in queue_families.iter() {
            // Sacrificing brevity for readability (thank me later)
            let queue_priorities_range = Range {
                start : priority_index as usize,
                end : (priority_index + count) as usize
            };
            priority_index += count;
            
            queue_create_infos.push(vk::DeviceQueueCreateInfo::default()
                .queue_family_index(family.index())
                .queue_priorities(&flat_queue_priorities[queue_priorities_range]));
        }

        let enabled_extension_names = extensions
            .iter()
            .map(|s| s.as_ptr())
            .collect::<Vec<_>>();

        
        let mut physical_device_descriptor_indexing_features = vk::PhysicalDeviceDescriptorIndexingFeatures::default();

        let mut physical_device_features2 = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut physical_device_descriptor_indexing_features);
        unsafe {
            instance.handle().get_physical_device_features2(self.handle, &mut physical_device_features2);
        }

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .push_next(&mut physical_device_features2)
            .enabled_extension_names(&enabled_extension_names);

    
        let device = unsafe {
            instance.handle().create_device(self.handle, &device_create_info, None)
                .expect("Failed to create a virtual device")
        };

        // Now, get all the queues
        let queues_objs = queue_families.iter().flat_map(|(count, family)| {
            (0..*count).map(|index| Queue::new(family, index, &device))
        }).collect::<Vec<_>>();

        Arc::new(LogicalDevice::new(instance,
            device,
            self.clone(),
            queues_objs,
            physical_device_features2.features,
            IndexingFeatures::new(physical_device_descriptor_indexing_features),
        ))
    }

    /// Creates a new [`PhysicalDevice`].
    /// 
    /// # Arguments
    /// 
    /// * `device` - The physical device backing this logical device.
    /// * `instance` - The global Vulkan instance.
    pub fn new(
        device : vk::PhysicalDevice,
        instance : &Arc<Context>
    ) -> Self {
        let physical_device_memory_properties = unsafe {
            instance.handle().get_physical_device_memory_properties(device)
        };

        let physical_device_properties = unsafe {
            instance.handle().get_physical_device_properties(device)
        };

        let queue_families = unsafe {
            instance.handle().get_physical_device_queue_family_properties(device)
        }.iter().enumerate().map(|(index, &properties)| QueueFamily::new(index as u32, properties)).collect::<Vec<_>>();

        Self {
            handle : device,
            context : Arc::downgrade(&instance),
            memory_properties : physical_device_memory_properties,
            properties : physical_device_properties,
            queue_families
        }
    }

    pub fn get_format_properties(&self, format : vk::Format) -> Option<vk::FormatProperties> {
        unsafe {
            let context = self.context.upgrade();
            if let Some(context) = context {
                return context.handle().get_physical_device_format_properties(self.handle, format).into();
            }
        }
        
        None
    }
}
