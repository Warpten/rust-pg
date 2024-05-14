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

    pub fn new(context : &Arc<Context>,
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
            context : context.clone(),
            queues,
            allocator : ManuallyDrop::new(Arc::new(Mutex::new(allocator)))
        }
    }

    /// Creates a new framebuffer for this logical device.
    /// 
    /// # Arguments
    /// 
    /// * `render_pass` - 
    /// * `extent` - 
    /// * `views` - A slice of image views used to create this framebuffer.
    /// * `layers` - 
    pub fn create_framebuffer(self : &Arc<Self>, render_pass : &Arc<RenderPass>, extent : ash::vk::Extent2D, views : &[ash::vk::ImageView], layers : u32) -> Framebuffer {
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

    /// Blocks until the completion of all operations of all queues on this logical device.
    pub fn wait_idle(&self) {
        unsafe {
            _ = self.handle.device_wait_idle();
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
    /// Indicates whether arrays of input attachments can be indexed by dynamically uniform integer expressions in shader code.
    /// If this feature is not enabled, resources with a descriptor type of VK_DESCRIPTOR_TYPE_INPUT_ATTACHMENT must be indexed
    /// only by constant integral expressions when aggregated into arrays in shader code. This also indicates whether shader
    /// modules can declare the InputAttachmentArrayDynamicIndexing capability.
    pub shader_input_attachment_array_dynamic_indexing: bool,

    /// Indicates whether arrays of uniform texel buffers can be indexed by dynamically uniform integer expressions in shader code.
    /// If this feature is not enabled, resources with a descriptor type of VK_DESCRIPTOR_TYPE_UNIFORM_TEXEL_BUFFER must be indexed
    /// only by constant integral expressions when aggregated into arrays in shader code. This also indicates whether shader
    /// modules can declare the UniformTexelBufferArrayDynamicIndexing capability.
    pub shader_uniform_texel_buffer_array_dynamic_indexing: bool,

    /// Indicates whether arrays of storage texel buffers can be indexed by dynamically uniform integer expressions in shader code.
    /// If this feature is not enabled, resources with a descriptor type of VK_DESCRIPTOR_TYPE_STORAGE_TEXEL_BUFFER must be indexed
    /// only by constant integral expressions when aggregated into arrays in shader code. This also indicates whether shader modules
    /// can declare the StorageTexelBufferArrayDynamicIndexing capability.
    pub shader_storage_texel_buffer_array_dynamic_indexing: bool,

    /// Indicates whether arrays of uniform buffers can be indexed by non-uniform integer expressions in shader code. If this feature
    /// is not enabled, resources with a descriptor type of VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER or VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER_DYNAMIC
    /// must not be indexed by non-uniform integer expressions when aggregated into arrays in shader code. This also indicates whether
    /// shader modules can declare the UniformBufferArrayNonUniformIndexing capability.
    pub shader_uniform_buffer_array_non_uniform_indexing: bool,

    /// Indicates whether arrays of samplers or sampled images can be indexed by non-uniform integer expressions in shader code.
    /// If this feature is not enabled, resources with a descriptor type of VK_DESCRIPTOR_TYPE_SAMPLER, VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
    /// or VK_DESCRIPTOR_TYPE_SAMPLED_IMAGE must not be indexed by non-uniform integer expressions when aggregated into arrays in shader
    /// code. This also indicates whether shader modules can declare the SampledImageArrayNonUniformIndexing capability.
    pub shader_sampled_image_array_non_uniform_indexing: bool,

    /// Indicates whether arrays of storage buffers can be indexed by non-uniform integer expressions in shader code. If this feature
    /// is not enabled, resources with a descriptor type of VK_DESCRIPTOR_TYPE_STORAGE_BUFFER or VK_DESCRIPTOR_TYPE_STORAGE_BUFFER_DYNAMIC
    /// must not be indexed by non-uniform integer expressions when aggregated into arrays in shader code. This also indicates whether
    /// shader modules can declare the StorageBufferArrayNonUniformIndexing capability.
    pub shader_storage_buffer_array_non_uniform_indexing: bool,

    /// Indicates whether arrays of storage images can be indexed by non-uniform integer expressions in shader code. If this feature is
    /// not enabled, resources with a descriptor type of VK_DESCRIPTOR_TYPE_STORAGE_IMAGE must not be indexed by non-uniform integer
    /// expressions when aggregated into arrays in shader code. This also indicates whether shader modules can declare the
    /// StorageImageArrayNonUniformIndexing capability.
    pub shader_storage_image_array_non_uniform_indexing: bool,

    /// Indicates whether arrays of input attachments can be indexed by non-uniform integer expressions in shader code. If this feature
    /// is not enabled, resources with a descriptor type of VK_DESCRIPTOR_TYPE_INPUT_ATTACHMENT must not be indexed by non-uniform
    /// integer expressions when aggregated into arrays in shader code. This also indicates whether shader modules can declare the
    /// InputAttachmentArrayNonUniformIndexing capability.
    pub shader_input_attachment_array_non_uniform_indexing: bool,

    /// Indicates whether arrays of uniform texel buffers can be indexed by non-uniform integer expressions in shader code. If this feature
    /// is not enabled, resources with a descriptor type of VK_DESCRIPTOR_TYPE_UNIFORM_TEXEL_BUFFER must not be indexed by non-uniform
    /// integer expressions when aggregated into arrays in shader code. This also indicates whether shader modules can declare the
    /// UniformTexelBufferArrayNonUniformIndexing capability.
    pub shader_uniform_texel_buffer_array_non_uniform_indexing: bool,

    /// Indicates whether arrays of storage texel buffers can be indexed by non-uniform integer expressions in shader code. If this feature
    /// is not enabled, resources with a descriptor type of VK_DESCRIPTOR_TYPE_STORAGE_TEXEL_BUFFER must not be indexed by non-uniform
    /// integer expressions when aggregated into arrays in shader code. This also indicates whether shader modules can declare the
    /// StorageTexelBufferArrayNonUniformIndexing capability.
    pub shader_storage_texel_buffer_array_non_uniform_indexing: bool,

    /// Indicates whether the implementation supports updating uniform buffer descriptors after a set is bound. If this feature is not enabled,
    /// VK_DESCRIPTOR_BINDING_UPDATE_AFTER_BIND_BIT must not be used with VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER.
    pub descriptor_binding_uniform_buffer_update_after_bind: bool,

    /// Indicates whether the implementation supports updating sampled image descriptors after a set is bound. If this feature is not enabled,
    /// VK_DESCRIPTOR_BINDING_UPDATE_AFTER_BIND_BIT must not be used with VK_DESCRIPTOR_TYPE_SAMPLER, VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
    /// or VK_DESCRIPTOR_TYPE_SAMPLED_IMAGE.
    pub descriptor_binding_sampled_image_update_after_bind: bool,

    /// Indicates whether the implementation supports updating storage image descriptors after a set is bound. If this feature is not enabled,
    /// VK_DESCRIPTOR_BINDING_UPDATE_AFTER_BIND_BIT must not be used with VK_DESCRIPTOR_TYPE_STORAGE_IMAGE.
    pub descriptor_binding_storage_image_update_after_bind: bool,

    /// Indicates whether the implementation supports updating storage buffer descriptors after a set is bound. If this feature is not enabled,
    /// VK_DESCRIPTOR_BINDING_UPDATE_AFTER_BIND_BIT must not be used with VK_DESCRIPTOR_TYPE_STORAGE_BUFFER.
    pub descriptor_binding_storage_buffer_update_after_bind: bool,

    /// Indicates whether the implementation supports updating uniform texel buffer descriptors after a set is bound. If this feature is not enabled,
    /// VK_DESCRIPTOR_BINDING_UPDATE_AFTER_BIND_BIT must not be used with VK_DESCRIPTOR_TYPE_UNIFORM_TEXEL_BUFFER.
    pub descriptor_binding_uniform_texel_buffer_update_after_bind: bool,

    /// Indicates whether the implementation supports updating storage texel buffer descriptors after a set is bound. If this feature is not enabled,
    /// VK_DESCRIPTOR_BINDING_UPDATE_AFTER_BIND_BIT must not be used with VK_DESCRIPTOR_TYPE_STORAGE_TEXEL_BUFFER.
    pub descriptor_binding_storage_texel_buffer_update_after_bind: bool,

    /// Indicates whether the implementation supports updating descriptors while the set is in use. If this feature is not enabled,
    /// VK_DESCRIPTOR_BINDING_UPDATE_UNUSED_WHILE_PENDING_BIT must not be used.
    pub descriptor_binding_update_unused_while_pending: bool,

    /// Indicates whether the implementation supports statically using a descriptor set binding in which some descriptors are not valid.
    /// If this feature is not enabled, VK_DESCRIPTOR_BINDING_PARTIALLY_BOUND_BIT must not be used.
    pub descriptor_binding_partially_bound: bool,

    /// Indicates whether the implementation supports descriptor sets with a variable-sized last binding. If this feature is not enabled,
    /// VK_DESCRIPTOR_BINDING_VARIABLE_DESCRIPTOR_COUNT_BIT must not be used.
    pub descriptor_binding_variable_descriptor_count: bool,

    /// Indicates whether the implementation supports the SPIR-V RuntimeDescriptorArray capability. If this feature is not enabled,
    /// descriptors must not be declared in runtime arrays.
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
