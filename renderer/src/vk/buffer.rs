use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::align_of;
use std::mem::replace;
use std::mem::size_of_val;
use ash::util::Align;
use ash::vk;
use gpu_allocator::vulkan::Allocation;
use gpu_allocator::vulkan::AllocationCreateDesc;
use gpu_allocator::vulkan::AllocationScheme;
use gpu_allocator::MemoryLocation;
use crate::make_handle;
use crate::orchestration::rendering::RenderingContext;
use crate::traits::handle::Handle;
use crate::vk::command_buffer::CommandBuffer;
use crate::vk::queue::QueueAffinity;

use super::command_pool::CommandPool;

pub struct StaticInitializerTag;
pub struct DynamicInitializerTag;

pub trait StaticInitializer {
    fn build(self, context : &RenderingContext, size : u64) -> Buffer;
}

pub trait DynamicInitializer {
    fn build<T : Sized + Copy>(self, context : &RenderingContext, pool : &CommandPool, data : &[T]) -> Buffer;
}

pub struct BufferBuilder<Tag> {
    name : &'static str,
    usage : vk::BufferUsageFlags,
    index_type : vk::IndexType,
    memory_location : MemoryLocation,
    linear : bool,
    sharing_mode : vk::SharingMode,

    _marker : PhantomData<Tag>,
}

pub type StaticBufferBuilder = BufferBuilder<StaticInitializerTag>;
pub type DynamicBufferBuilder = BufferBuilder<DynamicInitializerTag>;

impl<T> BufferBuilder<T> {
    pub fn fixed_size() -> BufferBuilder::<StaticInitializerTag> {
        BufferBuilder::<StaticInitializerTag> {
            name : Default::default(),
            usage : Default::default(),
            index_type : Default::default(),
            memory_location : MemoryLocation::Unknown,
            linear : Default::default(),
            sharing_mode : vk::SharingMode::EXCLUSIVE,

            _marker : PhantomData::default(),
        }
    }

    pub fn dynamic() -> BufferBuilder::<DynamicInitializerTag> {
        BufferBuilder::<DynamicInitializerTag> {
            name : Default::default(),
            usage : Default::default(),
            index_type : Default::default(),
            memory_location : MemoryLocation::Unknown,
            linear : Default::default(),
            sharing_mode : vk::SharingMode::EXCLUSIVE,

            _marker : PhantomData::default(),
        }
    }
}

impl StaticInitializer for BufferBuilder<StaticInitializerTag> {
    fn build(self, context : &RenderingContext, size : u64) -> Buffer {
        self.build_impl(context, size)
    }
}

impl DynamicInitializer for BufferBuilder<DynamicInitializerTag> {
    fn build<T : Sized + Copy>(self, context : &RenderingContext, pool : &CommandPool, data : &[T]) -> Buffer {
        let size = size_of_val(data) as u64;
        let mut this = self.build_impl(context, size);
        match &self.memory_location {
            MemoryLocation::GpuOnly => {
                let mut staging_buffer = StaticBufferBuilder::fixed_size()
                    .name("Staging buffer")
                    .cpu_to_gpu()
                    .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                    .build(context, size);
        
                staging_buffer.update(data);
        
                // Get the transfer queue.
                let transfer_queue = context.device.get_queue(QueueAffinity::Transfer, pool.family())
                    .expect("Failed to recover the transfer queue");
        
                // Begin a command buffer.
                let cmd = CommandBuffer::builder()
                    .pool(pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .build_one(context);
        
                cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
                cmd.label("Data upload to the GPU".to_owned(), [0.0; 4], || {
                    cmd.copy_buffer(&staging_buffer, &this, &[vk::BufferCopy::default()
                        .size(size)
                    ]);
                });
                cmd.end();
        
                context.device.submit(transfer_queue, &[&cmd], &[], &[], vk::Fence::null());
                unsafe {
                    context.device.handle().queue_wait_idle(transfer_queue.handle())
                        .expect("Waiting for queue idle failed");
                }
        
                this.element_count = data.len() as _;
            }
            _ => this.update(data)
        }

        this
    }
}

impl<T> BufferBuilder<T> {
    value_builder! { sharing_mode, vk::SharingMode }
    value_builder! { name, name, &'static str }
    value_builder! { index, index_type, vk::IndexType }
    value_builder! { linear, linear, bool }

    #[inline] pub fn usage(mut self, usage : vk::BufferUsageFlags) -> Self {
        self.usage = usage;
        if usage == vk::BufferUsageFlags::VERTEX_BUFFER {
            self.linear = true;
        }
        self
    }

    valueless_builder! { gpu_only,   MemoryLocation::GpuOnly }
    valueless_builder! { cpu_to_gpu, MemoryLocation::CpuToGpu }
    valueless_builder! { gpu_to_cpu, MemoryLocation::GpuToCpu }

    pub(in self) fn build_impl(&self, context : &RenderingContext, size : u64) -> Buffer {
        unsafe {
            assert!(size != 0, "A buffer with no capacity is probably not what you want.");
            
            let mut usage = self.usage;
            // If we don't do this we can never write to this buffer
            if self.memory_location == MemoryLocation::GpuOnly {
                usage |= vk::BufferUsageFlags::TRANSFER_DST;
            }

            let create_info = vk::BufferCreateInfo::default()
                .usage(usage)
                .sharing_mode(self.sharing_mode)
                .size(size);

            let buffer = context.device.handle().create_buffer(&create_info, None)
                .expect("Buffer creation failed");
            
            if !self.name.is_empty() {
                context.device.set_handle_name(buffer, &self.name.to_owned());
            }

            let requirements = context.device.handle().get_buffer_memory_requirements(buffer);
            
            let allocation = context.device.allocator()
                .lock()
                .unwrap()
                .allocate(&AllocationCreateDesc {
                    name : self.name,
                    requirements,
                    linear : self.linear,
                    location : self.memory_location,
                    allocation_scheme : AllocationScheme::GpuAllocatorManaged
                })
                .expect("Buffer memory allocation failed");

            context.device.handle().bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .expect("Binding buffer memory failed");

            Buffer {
                context : context.clone(),
                handle : buffer,
                allocation,
                index_type : self.index_type,
                element_count : 0
            }
        }
    }
}

pub struct Buffer {
    context : RenderingContext,
    handle : vk::Buffer,
    allocation : Allocation,
    index_type : vk::IndexType,
    element_count : u32,
}

impl Buffer {
    pub fn update<T : Copy>(&mut self, data : &[T]) {
        let size = size_of_val(data) as u64;
        assert!(self.allocation.size() >= size, "The data you're trying to write to the buffer is too large to fit.");

        unsafe {
            let mapped_data = self.allocation.mapped_ptr()
                .expect("This memory allocation should be host visible. If it can't be, consider using a staging buffer.")
                .as_ptr();
            let mut mapping_slice = Align::new(
                mapped_data as *mut c_void,
                align_of::<T>() as u64,
                size
            );
            mapping_slice.copy_from_slice(data);
        }

        self.element_count = data.len() as u32;
    }

    pub fn map(&self) -> *mut u8 {
        self.allocation.mapped_ptr().unwrap().as_ptr() as *mut u8
    }

    pub fn element_count(&self) -> u32 {
        self.element_count
    }

    pub unsafe fn memory(&self) -> vk::DeviceMemory {
        self.allocation.memory()
    }

    pub fn get_device_address(&self) -> u64 {
        unsafe {
            self.context.device.handle().get_buffer_device_address(&vk::BufferDeviceAddressInfo::default()
                .buffer(self.handle))
        }
    }

    #[inline] pub fn index_type(&self) -> vk::IndexType { self.index_type }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.context.device.handle().destroy_buffer(self.handle, None);

            let memory = replace(&mut self.allocation, Allocation::default()); 
            _ = self.context.device.allocator().lock().unwrap().free(memory);
        }

    }
}

make_handle! { Buffer, vk::Buffer }