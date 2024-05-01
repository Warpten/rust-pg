use std::ops::{Deref, Index};

use ash::vk;
use gpu_allocator::MemoryLocation;

use super::Context;

pub struct BufferInfo<'a> {
    pub name : &'a str,
    pub usage : vk::BufferUsageFlags,
    pub location : MemoryLocation,
    pub memory_type_bits : Option<u32>,
}

impl<'a> BufferInfo<'a> {
    pub fn name(mut self, name: &'a str ) -> Self {
        self.name = name;
        self
    }
    pub fn usage(mut self, usage: vk::BufferUsageFlags) -> Self {
        self.usage = usage;
        self
    }
    pub fn usage_transfer_src(mut self) -> Self {
        self.usage |= vk::BufferUsageFlags::TRANSFER_SRC;
        self
    }
    pub fn usage_transfer_dst(mut self) -> Self {
        self.usage |= vk::BufferUsageFlags::TRANSFER_DST;
        self
    }
    pub fn usage_uniform_texel(mut self) -> Self {
        self.usage |= vk::BufferUsageFlags::UNIFORM_TEXEL_BUFFER;
        self
    }
    pub fn usage_storage_texel(mut self) -> Self {
        self.usage |= vk::BufferUsageFlags::STORAGE_TEXEL_BUFFER;
        self
    }
    pub fn usage_uniform(mut self) -> Self {
        self.usage |= vk::BufferUsageFlags::UNIFORM_BUFFER;
        self
    }
    pub fn usage_storage(mut self) -> Self {
        self.usage |= vk::BufferUsageFlags::STORAGE_BUFFER;
        self
    }
    pub fn usage_index(mut self) -> Self {
        self.usage |= vk::BufferUsageFlags::INDEX_BUFFER;
        self
    }
    pub fn usage_vertex(mut self) -> Self {
        self.usage |= vk::BufferUsageFlags::VERTEX_BUFFER;
        self
    }
    pub fn usage_indirect(mut self) -> Self {
        self.usage |= vk::BufferUsageFlags::INDIRECT_BUFFER;
        self
    }
    pub fn gpu_only(mut self) -> Self {
        self.mem_usage = MemoryLocation::GpuOnly;
        self
    }
    pub fn cpu_to_gpu(mut self) -> Self {
        self.mem_usage = MemoryLocation::CpuToGpu;
        self
    }
    pub fn gpu_to_cpu(mut self) -> Self {
        self.mem_usage = MemoryLocation::GpuToCpu;
        self
    }
    pub fn memory_type_bits(mut self, memory_type_bits: u32) -> Self {
        self.memory_type_bits = Some(memory_type_bits);
        self
    }
}

impl Default for BufferInfo<'_> {
    fn default() -> Self {
        Self {
            name: "Buffer",
            usage: Default::default(),
            location: MemoryLocation::CpuToGpu,
            memory_type_bits: None
        }
    }
}

pub struct IndexBufferInfo<'a> {
    pub base : BufferInfo<'a>,
    pub index_type : Option<ash::vk::IndexType>
}

impl<'a> Deref for IndexBufferInfo<'a> {
    type Target = BufferInfo<'a>;

    fn deref(&self) -> &BufferInfo<'a> { &self.base }
}

impl<'a> IndexBufferInfo<'a> {
    pub fn index_type(mut self, index_type: vk::IndexType) -> Self {
        self.index_type = Some(index_type);
        self
    }
}

impl Default for IndexBufferInfo<'_> {
    fn default() -> Self {
        Self {
            base: Default::default(),
            index_type: None
        }
    }
}

struct Buffer {
    context : Arc<Context>,
}
