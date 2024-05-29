use std::sync::Arc;

use ash::vk;
use crate::vk::command_pool::CommandPool;
use crate::vk::logical_device::LogicalDevice;
use crate::vk::semaphore_pool::SemaphorePool;

use super::command_buffer::CommandBuffer;
use super::queue::QueueAffinity;

pub struct FrameData {
    device : Arc<LogicalDevice>,
    pub index : usize,
    pub semaphore_pool : SemaphorePool,
    pub in_flight : vk::Fence,

    pub graphics_command_pool : Option<CommandPool>,
}

impl FrameData {
    pub fn new(index : usize, device : &Arc<LogicalDevice>) -> Self {
        Self {
            device : device.clone(),
            index,
            in_flight : device.create_fence(vk::FenceCreateFlags::SIGNALED),
            semaphore_pool : SemaphorePool::new(device),
            graphics_command_pool : device.get_queues(QueueAffinity::Graphics)
                .first()
                .map(|queue| queue.family())
                .map(|family| {
                    CommandPool::builder(family)
                        .reset()
                        .build(device)
                }),
        }
    }

    pub fn make_command_buffer(&self, level : vk::CommandBufferLevel) -> CommandBuffer {
        CommandBuffer::builder()
            .level(level)
            .pool(self.graphics_command_pool.as_ref().unwrap())
            .build_one(&self.device)
    }
}

impl Drop for FrameData {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_fence(self.in_flight, None);
        }
    }
}