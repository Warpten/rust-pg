use std::ffi::c_void;
use std::mem::align_of;
use std::mem::size_of_val;
use std::sync::Arc;
use ash::util::Align;
use ash::vk;
use gpu_allocator::vulkan::Allocation;
use gpu_allocator::vulkan::AllocationCreateDesc;
use gpu_allocator::vulkan::AllocationScheme;
use gpu_allocator::MemoryLocation;
use crate::traits::handle::Handle;
use crate::vk::logical_device::LogicalDevice;

pub enum BufferInitialization<'a, T : Sized + Copy> {
    Zeroed(u64),
    From(&'a [T]),
}

pub struct BufferBuilder<'a, T : Sized + Copy> {
    name : &'static str,
    usage : vk::BufferUsageFlags,
    index_type : vk::IndexType,
    memory_location : MemoryLocation,
    initialization : BufferInitialization<'a, T>,
    linear : bool,
}

impl<T : Sized + Copy> Default for BufferBuilder<'_, T> {
    fn default() -> Self {
        Self {
            name: Default::default(),
            usage: Default::default(),
            index_type: Default::default(),
            memory_location: MemoryLocation::Unknown,
            initialization: BufferInitialization::Zeroed(0),
            linear: Default::default()
        }
    }
}

impl<'a, T : Sized + Copy> BufferBuilder<'a, T> {
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

    #[inline] pub fn data(mut self, data : &'a [T]) -> Self {
        self.initialization = BufferInitialization::From(data);
        self
    }

    #[inline] pub fn zeroed(mut self, size : u64) -> Self {
        self.initialization = BufferInitialization::Zeroed(size);
        self
    }

    pub fn build(self, device : &Arc<LogicalDevice>) -> Buffer {
        unsafe {
            let size = match &self.initialization {
                BufferInitialization::Zeroed(size) => *size,
                BufferInitialization::From(arr) => size_of_val(*arr) as u64,
            };

            assert!(size != 0, "A buffer with no capacity is probably not what you want.");
            
            let mut usage = self.usage;
            // If we don't do this we can never write to this buffer
            if self.memory_location == MemoryLocation::GpuOnly {
                usage |= vk::BufferUsageFlags::TRANSFER_DST;
            }

            let create_info = vk::BufferCreateInfo::default()
                .usage(usage)
                .size(size);

            let buffer = device.handle().create_buffer(&create_info, None)
                .expect("Buffer creation failed");

            let requirements = device.handle().get_buffer_memory_requirements(buffer);
            
            let allocation = device.allocator()
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

            device.handle().bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .expect("Binding buffer memory failed");

            let this = Buffer {
                device : device.clone(),
                handle : buffer,
                allocation,
                index_type : self.index_type
            };

            if let BufferInitialization::From(data) = self.initialization {
                match self.memory_location {
                    MemoryLocation::GpuOnly => {
                        let staging_buffer = Self::default()
                            .name("Staging buffer")
                            .cpu_to_gpu()
                            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                            .zeroed(size)
                            .build(device);

                        staging_buffer.update(data);

                        // Begin a single time command buffer
                        // and send out a vkCmdCopyBuffer.
                        todo!("vkCmdCopyBuffer is not implemented");
                    },
                    _ => this.update(data),
                }
            }

            this
        }
    }
}

pub struct Buffer {
    device : Arc<LogicalDevice>,
    handle : vk::Buffer,
    allocation : Allocation,
    index_type : vk::IndexType,
}

impl Buffer {
    pub fn update<T : Copy>(&self, data : &[T]) {
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
    }

    pub fn map(&self) -> *mut u8 {
        self.allocation.mapped_ptr().unwrap().as_ptr() as *mut u8
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_buffer(self.handle, None);
        }
    }
}

impl Handle<vk::Buffer> for Buffer {
    fn handle(&self) -> vk::Buffer { self.handle }
}