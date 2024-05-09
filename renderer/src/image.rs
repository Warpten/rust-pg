use std::sync::Arc;

use crate::{traits::BorrowHandle, LogicalDevice, QueueFamily};

pub struct Image {
    device : Arc<LogicalDevice>,
    handle : ash::vk::Image,
    allocation : ash::vk::DeviceMemory,
}

impl Image {
    /// Creates a new image.
    /// 
    /// # Arguments
    /// 
    /// * `device` - The logical device owning this image.
    /// * `format` - Describes the format and type of the texel blocks that will be contained in the image.
    /// * `extent` - Number of data elements in each dimension of the image.
    /// * `levels` - Number of levels of detail available for minified sampling of the image.
    /// * `layers` - Number of layers in the image.
    /// * `sampling` - The number of [samples per texel](https://registry.khronos.org/vulkan/specs/1.3-extensions/html/vkspec.html#primsrast-multisampling).
    /// * `tiling` - The tiling arrangement of the texel blocks in memory.
    /// * `usage` - The intended usage of the image.
    /// * `initial_layout` - The initial layout of this image.
    /// * `queues` - All queue families allowed to access this image.
    pub fn new(
        device : Arc<LogicalDevice>,
        format : ash::vk::Format,
        extent : ash::vk::Extent3D,
        levels : u32,
        layers : u32,
        sampling : ash::vk::SampleCountFlags,
        tiling : ash::vk::ImageTiling,
        usage : ash::vk::ImageUsageFlags,
        initial_layout : ash::vk::ImageLayout,
        queues : &[QueueFamily]
    ) -> Self
    {
        let queue_families = queues.iter().map(|q| q.index).collect::<Vec<_>>();
        let sharing_mode = match queue_families.len() {
            1 => ash::vk::SharingMode::EXCLUSIVE,
            _ => ash::vk::SharingMode::CONCURRENT,
        };

        let image_create_info = ash::vk::ImageCreateInfo::default()
            .format(format)
            .extent(extent)
            .mip_levels(levels)
            .array_layers(layers)
            .samples(sampling)
            .sharing_mode(sharing_mode)
            .initial_layout(initial_layout)
            .tiling(tiling)
            .queue_family_indices(&queue_families[..])
            .usage(usage);

        let handle = unsafe {
            device.handle().create_image(&image_create_info, None)
                .expect("Failed to create image")
        };

        let memory_requirements = unsafe { device.handle().get_image_memory_requirements(handle) };

        let memory_allocation = ash::vk::MemoryAllocateInfo::default()
            .allocation_size(memory_requirements.size)
            .memory_type_index(device.find_memory_type(memory_requirements.memory_type_bits, ash::vk::MemoryPropertyFlags::DEVICE_LOCAL));

        let allocation = unsafe {
            device.handle().allocate_memory(&memory_allocation, None)
                .expect("Memory allocation failed")
        };

        unsafe {
            device.handle().bind_image_memory(handle, allocation, 0).expect("Binding image memory failed")
        };

        Self { device, handle, allocation }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().free_memory(self.allocation, None);
            self.device.handle().destroy_image(self.handle, None);
        }
    }
}