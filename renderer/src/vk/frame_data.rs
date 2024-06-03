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
    pub(in crate) image_available : vk::Semaphore,
    pub(in crate) render_finished : vk::Semaphore,

    pub graphics_command_pool : CommandPool,
    pub cmd : CommandBuffer,
}

impl FrameData {
    pub fn new(index : usize, device : &Arc<LogicalDevice>) -> Self {
        let graphics_queue = device.get_queue(QueueAffinity::Graphics, 0).unwrap();
        let graphics_command_pool = CommandPool::builder(graphics_queue.family())
            .reset()
            .build(device);

        let cmd = CommandBuffer::builder()
            .pool(&graphics_command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .build_one(device);

        Self {
            device : device.clone(),
            index,
            in_flight : device.create_fence(vk::FenceCreateFlags::SIGNALED, format!("Frame in flight fence {}", index).into()),
            semaphore_pool : SemaphorePool::new(device),
            graphics_command_pool,
            cmd,
            image_available : device.create_semaphore(),
            render_finished : device.create_semaphore(),
        }
    }

    pub fn make_command_buffer(&self, level : vk::CommandBufferLevel) -> CommandBuffer {
        CommandBuffer::builder()
            .level(level)
            .pool(&self.graphics_command_pool)
            .build_one(&self.device)
    }
}

impl Drop for FrameData {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_semaphore(self.image_available, None);
            self.device.handle().destroy_semaphore(self.render_finished, None);
            self.device.handle().destroy_fence(self.in_flight, None);
        }
    }
}