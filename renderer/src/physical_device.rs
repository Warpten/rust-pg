use std::{collections::HashMap, ffi::CString, ops::Range, sync::{Arc, Weak}};

use crate::traits::{BorrowHandle, Handle};

use super::{QueueFamily, Context, LogicalDevice, Queue};

#[derive(Clone)]
pub struct PhysicalDevice {
    handle : ash::vk::PhysicalDevice,
    context : Weak<Context>,
    memory_properties : ash::vk::PhysicalDeviceMemoryProperties,
    properties : ash::vk::PhysicalDeviceProperties,
    pub queue_families : Vec<QueueFamily>,
}

impl Handle for PhysicalDevice {
    type Target = ash::vk::PhysicalDevice;

    fn handle(&self) -> ash::vk::PhysicalDevice { self.handle }
}

impl PhysicalDevice {
    #[inline] pub fn context(&self) -> &Weak<Context> { &self.context }
    #[inline] pub fn memory_properties(&self) -> &ash::vk::PhysicalDeviceMemoryProperties { &self.memory_properties }
    #[inline] pub fn properties(&self) -> &ash::vk::PhysicalDeviceProperties { &self.properties }

    /// Returns the extensions available on this [`PhysicalDevice`].
    ///
    /// # Panics
    ///
    /// Panics if [`vkEnumerateDeviceExtensionProperties`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkEnumerateDeviceExtensionProperties.html) fails.
    pub fn get_extensions(&self) -> Vec<ash::vk::ExtensionProperties> {
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
        instance : Arc<Context>,
        queue_families : Vec<(u32, QueueFamily)>,
        get_queue_priority : F,
        extensions : Vec<CString>,
    ) -> Arc<LogicalDevice>
        where F : Fn(u32, QueueFamily) -> f32
    {
        // If we have the same queue family twice (for example, a graphics queue family and a present-able queue family are the same)
        // deduplicate them.
        // That implies we must sum up the queue counts - we'll limit them later
        let queue_families = {
            let mut work_buffer = HashMap::<QueueFamily, u32>::with_capacity(queue_families.len());
            for (count, family) in queue_families {
                work_buffer.entry(family).and_modify(|v| { *v += count; }).or_insert(count);
            }
            // Short lived, don't shrink_to_fit
            work_buffer
        };

        // Store queue priorities in a flattened buffer; each queue family will index into
        // that buffer to slice out the amount of queue families.
        let mut queue_create_infos = Vec::with_capacity(queue_families.len());
        let mut flat_queue_priorities = vec![];

        // Unfortunately has to happen in two loops because one borrow is immutable
        // and the other is mutable...
        for (&key, &value) in &queue_families {
            for queue_index in 0..value.min(key.properties.queue_count) {
                flat_queue_priorities.push(get_queue_priority(queue_index, key));
            }
        }
        
        for (&key, &value) in &queue_families {
            let priority_index = flat_queue_priorities.len();
            // Sacrificing brevity for readability (thank me later)
            let queue_priorities_range = Range {
                start : priority_index,
                end : priority_index + (value.min(key.properties.queue_count) as usize)
            };
            
            queue_create_infos.push(ash::vk::DeviceQueueCreateInfo::default()
                .queue_family_index(key.index)
                .queue_priorities(&flat_queue_priorities[queue_priorities_range]));
        }

        let physical_device_features = unsafe {
            instance.handle().get_physical_device_features(self.handle)
        };

        let enabled_extension_names = extensions
            .iter()
            .map(|s| s.as_ptr())
            .collect::<Vec<_>>();

        let mut physical_device_descriptor_indexing_features = ash::vk::PhysicalDeviceDescriptorIndexingFeatures::default();

        let mut physical_device_features2 = ash::vk::PhysicalDeviceFeatures2::default();
        _ = physical_device_features2.push_next(&mut physical_device_descriptor_indexing_features);
        unsafe {
            instance.handle().get_physical_device_features2(self.handle, &mut physical_device_features2);
        }

        assert_ne!(physical_device_descriptor_indexing_features.shader_sampled_image_array_non_uniform_indexing, 0);
        assert_ne!(physical_device_descriptor_indexing_features.descriptor_binding_sampled_image_update_after_bind, 0);
        assert_ne!(physical_device_descriptor_indexing_features.shader_uniform_buffer_array_non_uniform_indexing, 0);
        assert_ne!(physical_device_descriptor_indexing_features.descriptor_binding_uniform_buffer_update_after_bind, 0);
        assert_ne!(physical_device_descriptor_indexing_features.shader_storage_buffer_array_non_uniform_indexing, 0);
        assert_ne!(physical_device_descriptor_indexing_features.descriptor_binding_storage_buffer_update_after_bind, 0);

        let device_create_info = ash::vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .push_next(&mut physical_device_features2)
            .enabled_features(&physical_device_features)
            .enabled_extension_names(&enabled_extension_names);

        let device = unsafe {
            instance.handle().create_device(self.handle, &device_create_info, None)
                .expect("Failed to create a virtual device")
        };

        // Now, get all the queues
        let queues_objs = queue_families.iter().flat_map(|(family, count)| {
            (0..*count).map(|index| Queue::new(*family, index, &device))
        }).collect::<Vec<_>>();

        Arc::new(LogicalDevice::new(instance, device, self.clone(), queues_objs))
    }

    /// Creates a new [`PhysicalDevice`].
    /// 
    /// # Arguments
    /// 
    /// * `device` - The physical device backing this logical device.
    /// * `instance` - The global Vulkan instance.
    pub fn new(
        device : ash::vk::PhysicalDevice,
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
        }.iter().enumerate().map(|(index, &properties)| {
            QueueFamily {  index: index as u32, properties }
        }).collect::<Vec<_>>();

        Self {
            handle : device,
            context : Arc::downgrade(&instance),
            memory_properties : physical_device_memory_properties,
            properties : physical_device_properties,
            queue_families
        }
    }
}
