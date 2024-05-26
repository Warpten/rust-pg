use std::collections::HashMap;
use std::sync::Arc;
use ash::vk;
use nohash_hasher::IntMap;

use crate::make_handle;
use crate::traits::handle::Handle;
use crate::vk::descriptor::set::DescriptorSetInfo;
use crate::vk::logical_device::LogicalDevice;
use crate::vk::renderer::Renderer;

pub struct PoolDescriptorCount(pub u32);
pub struct BindingDescriptorCount(pub u32);

/// Facililates creating instances of [`DescriptorSetLayout`].
pub struct DescriptorSetLayoutBuilder {
    pub(in self) bindings : IntMap<u32, (vk::DescriptorType, vk::ShaderStageFlags, PoolDescriptorCount, BindingDescriptorCount)>,
    pub(in self) flags : vk::DescriptorSetLayoutCreateFlags,
    pub(in self) sets : u32,
}

impl DescriptorSetLayoutBuilder {
    #[inline] pub fn binding(mut self, binding : u32, descriptor_type : vk::DescriptorType, stage : vk::ShaderStageFlags,
        pool_descriptor_count : PoolDescriptorCount,
        binding_descriptor_count : BindingDescriptorCount
    ) -> Self {
        self.bindings.insert(binding, (descriptor_type, stage, pool_descriptor_count, binding_descriptor_count));
        self
    }

    value_builder! { sets, count, sets, u32 }
    value_builder! { flags, vk::DescriptorSetLayoutCreateFlags }

    pub fn build(mut self, renderer : &Renderer) -> DescriptorSetLayout {
        DescriptorSetLayout::new(&renderer.device, self)
    }
}

impl Default for DescriptorSetLayoutBuilder {
    fn default() -> Self {
        Self {
            sets: 64,
            bindings : IntMap::default(),
            flags : vk::DescriptorSetLayoutCreateFlags::empty(),
        }
    }
}

/// A somewhat thin wrapped around [`vk::DescriptorSetLayout`]. This object also manages a pool of descriptors as well
/// as known descriptor sets.
/// 
/// To instanciate this class, see [`DescriptorSetLayoutBuilder`]
pub struct DescriptorSetLayout {
    device : Arc<LogicalDevice>,
    layout : vk::DescriptorSetLayout,
    pool : vk::DescriptorPool,

    // Store the info used to build this object.
    // TODO: Make this go away.
    info : DescriptorSetLayoutBuilder,

    sets : HashMap<DescriptorSetInfo, vk::DescriptorSet>,
}

impl DescriptorSetLayout {
    pub(in self) fn new(device : &Arc<LogicalDevice>, info : DescriptorSetLayoutBuilder) -> Self {
        let binding_count = info.bindings.len();
        let mut bindings = Vec::<vk::DescriptorSetLayoutBinding>::with_capacity(binding_count);
        let mut pool_sizes = Vec::<vk::DescriptorPoolSize>::with_capacity(binding_count);

        for (binding, (descriptor_type, stage_flags, pool_descriptor_count, binding_descriptor_count)) in &info.bindings {
            bindings.push(vk::DescriptorSetLayoutBinding::default()
                .binding(*binding)
                .descriptor_type(*descriptor_type)
                .stage_flags(*stage_flags)
                .descriptor_count(binding_descriptor_count.0)
            );

            pool_sizes.push(vk::DescriptorPoolSize::default()
                .ty(*descriptor_type)
                .descriptor_count(pool_descriptor_count.0)
            );
        }

        unsafe {
            let create_info = vk::DescriptorSetLayoutCreateInfo::default()
                .flags(info.flags)
                .bindings(&bindings);

            let layout = device.handle()
                .create_descriptor_set_layout(&create_info, None)
                .expect("Descriptor set layout creation failed");

            let pool_create_info = vk::DescriptorPoolCreateInfo::default()
                .max_sets(info.sets)
                .pool_sizes(&pool_sizes)
                .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET);

            let pool = device.handle()
                .create_descriptor_pool(&pool_create_info, None)
                .expect("Descriptor pool creation failed");

            Self {
                device : device.clone(),
                layout,
                pool,
                info,
                sets : HashMap::new(),
            }
        }
    }

    pub fn request(&mut self, info : DescriptorSetInfo) -> vk::DescriptorSet {
        assert!(!info.is_empty(), "Can't request an empty descriptor set");

        unsafe {
            let value = self.sets.get(&info);
            match value {
                Some(value) => *value,
                None => {
                    let handle = self.device.handle()
                        .allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo::default()
                            .descriptor_pool(self.pool)
                            .set_layouts(&[self.layout])
                        )
                        .expect("Descriptor set allocation failed")[0];

                    self.update_sets(handle, &info);
                    self.sets.insert(info, handle.clone());
                    handle
                }
            }
        }
    }

    fn update_sets(&mut self, set : vk::DescriptorSet, info : &DescriptorSetInfo) {
        let capacity = info.buffers.len() + info.images.len();
        let mut write_sets = Vec::<vk::WriteDescriptorSet>::with_capacity(capacity);

        for (binding, info) in &info.buffers {
            write_sets.push(vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(*binding)
                .dst_array_element(0)
                .descriptor_type(self.info.bindings[binding].0)
                .buffer_info(&info[..])
            );
        }

        for (binding, info) in &info.images {
            write_sets.push(vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(*binding)
                .dst_array_element(0)
                .descriptor_type(self.info.bindings[binding].0)
                .image_info(&info[..])
            );
        }

        unsafe {
            self.device.handle()
                .update_descriptor_sets(&write_sets, &[]);
        }
    }

    pub fn reset_pool(&self) {
        unsafe {
            self.device.handle()
                .reset_descriptor_pool(self.pool, vk::DescriptorPoolResetFlags::default())
                .expect("Failed to reset descriptor pool.");
        }
    }

    pub fn get_descriptor_type(&self, binding : u32) -> vk::DescriptorType {
        self.info.bindings[&binding].0
    }

    pub fn get_shader_stage(&self, binding : u32) -> vk::ShaderStageFlags {
        self.info.bindings[&binding].1
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        unsafe {
            self.device.handle()
                .destroy_descriptor_set_layout(self.layout, None);
            self.device.handle()
                .destroy_descriptor_pool(self.pool, None);
        }
    }
}

make_handle! { DescriptorSetLayout, vk::DescriptorSetLayout, layout }
