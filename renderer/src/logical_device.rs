use std::{mem::ManuallyDrop, sync::{Arc, Mutex}};

use gpu_allocator::{vulkan::{Allocator, AllocatorCreateDesc}, AllocationSizes, AllocatorDebugSettings};

use crate::{traits::{BorrowHandle, Handle}, Framebuffer, RenderPass};

use super::{Queue, Context, PhysicalDevice};

/// A logical Vulkan device.
pub struct LogicalDevice {
    handle : ash::Device,
    context : Arc<Context>,
    physical_device : PhysicalDevice,
    allocator : ManuallyDrop<Arc<Mutex<Allocator>>>,

    pub queues : Vec<Queue>,

    pub features : ash::vk::PhysicalDeviceFeatures,
    pub indexing_features : IndexingFeatures,
}

impl LogicalDevice {
    pub fn context(&self) -> &Arc<Context> { &self.context }
    pub fn physical_device(&self) -> &PhysicalDevice { &self.physical_device }
    pub fn allocator(&self) -> &Arc<Mutex<Allocator>> { &self.allocator }

    pub fn new(context : Arc<Context>,
        device : ash::Device,
        physical_device : PhysicalDevice,
        queues : Vec<Queue>,
        features : ash::vk::PhysicalDeviceFeatures,
        indexing_features : IndexingFeatures,
    )  -> Self {
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
            features,
            indexing_features,
            context,
            queues,
            allocator : ManuallyDrop::new(Arc::new(Mutex::new(allocator)))
        }
    }

    /// Creates a new framebuffer
    /// 
    /// # Arguments
    /// 
    /// * `render_pass` - 
    /// * `extent` - 
    /// * `views` - A slice of image views used to create this framebuffer.
    /// * `layers` - 
    pub fn create_framebuffer(self : Arc<Self>, render_pass : &Arc<RenderPass>, extent : ash::vk::Extent2D, views : Vec<ash::vk::ImageView>, layers : u32) -> Framebuffer {
        return Framebuffer::new(extent, views, layers, self, render_pass)
    }

    pub fn find_memory_type(&self, memory_type_bits : u32, flags : ash::vk::MemoryPropertyFlags) -> u32 {
        for (i, memory_type) in self.physical_device().memory_properties().memory_types.iter().enumerate() {
            if (memory_type_bits & (1 << i)) != 0 && (memory_type.property_flags & flags) == flags {
                return i as _;
            }
        }

        panic!("No memory type found matching the requirements")
    }

    pub fn wait_idle(&self) {
        unsafe {
            self.handle.device_wait_idle();
        }
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


pub struct IndexingFeatures {
    pub shader_input_attachment_array_dynamic_indexing: bool,
    pub shader_uniform_texel_buffer_array_dynamic_indexing: bool,
    pub shader_storage_texel_buffer_array_dynamic_indexing: bool,
    pub shader_uniform_buffer_array_non_uniform_indexing: bool,
    pub shader_sampled_image_array_non_uniform_indexing: bool,
    pub shader_storage_buffer_array_non_uniform_indexing: bool,
    pub shader_storage_image_array_non_uniform_indexing: bool,
    pub shader_input_attachment_array_non_uniform_indexing: bool,
    pub shader_uniform_texel_buffer_array_non_uniform_indexing: bool,
    pub shader_storage_texel_buffer_array_non_uniform_indexing: bool,
    pub descriptor_binding_uniform_buffer_update_after_bind: bool,
    pub descriptor_binding_sampled_image_update_after_bind: bool,
    pub descriptor_binding_storage_image_update_after_bind: bool,
    pub descriptor_binding_storage_buffer_update_after_bind: bool,
    pub descriptor_binding_uniform_texel_buffer_update_after_bind: bool,
    pub descriptor_binding_storage_texel_buffer_update_after_bind: bool,
    pub descriptor_binding_update_unused_while_pending: bool,
    pub descriptor_binding_partially_bound: bool,
    pub descriptor_binding_variable_descriptor_count: bool,
    pub runtime_descriptor_array: bool,
}

impl IndexingFeatures {
    pub fn new(features : ash::vk::PhysicalDeviceDescriptorIndexingFeatures) -> Self {
        Self {
            shader_input_attachment_array_dynamic_indexing : features.shader_input_attachment_array_dynamic_indexing != 0,
            shader_uniform_texel_buffer_array_dynamic_indexing : features.shader_uniform_texel_buffer_array_dynamic_indexing != 0,
            shader_storage_texel_buffer_array_dynamic_indexing : features.shader_storage_texel_buffer_array_dynamic_indexing != 0,
            shader_uniform_buffer_array_non_uniform_indexing : features.shader_uniform_buffer_array_non_uniform_indexing != 0,
            shader_sampled_image_array_non_uniform_indexing : features.shader_sampled_image_array_non_uniform_indexing != 0,
            shader_storage_buffer_array_non_uniform_indexing : features.shader_storage_buffer_array_non_uniform_indexing != 0,
            shader_storage_image_array_non_uniform_indexing : features.shader_storage_image_array_non_uniform_indexing != 0,
            shader_input_attachment_array_non_uniform_indexing : features.shader_input_attachment_array_non_uniform_indexing != 0,
            shader_uniform_texel_buffer_array_non_uniform_indexing : features.shader_uniform_texel_buffer_array_non_uniform_indexing != 0,
            shader_storage_texel_buffer_array_non_uniform_indexing : features.shader_storage_texel_buffer_array_non_uniform_indexing != 0,
            descriptor_binding_uniform_buffer_update_after_bind : features.descriptor_binding_uniform_buffer_update_after_bind != 0,
            descriptor_binding_sampled_image_update_after_bind : features.descriptor_binding_sampled_image_update_after_bind != 0,
            descriptor_binding_storage_image_update_after_bind : features.descriptor_binding_storage_image_update_after_bind != 0,
            descriptor_binding_storage_buffer_update_after_bind : features.descriptor_binding_storage_buffer_update_after_bind != 0,
            descriptor_binding_uniform_texel_buffer_update_after_bind : features.descriptor_binding_uniform_texel_buffer_update_after_bind != 0,
            descriptor_binding_storage_texel_buffer_update_after_bind : features.descriptor_binding_storage_texel_buffer_update_after_bind != 0,
            descriptor_binding_update_unused_while_pending : features.descriptor_binding_update_unused_while_pending != 0,
            descriptor_binding_partially_bound : features.descriptor_binding_partially_bound != 0,
            descriptor_binding_variable_descriptor_count : features.descriptor_binding_variable_descriptor_count != 0,
            runtime_descriptor_array : features.runtime_descriptor_array != 0,
        }
    }
}
