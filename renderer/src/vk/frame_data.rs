use std::sync::Arc;

use ash::vk;
use nohash_hasher::IntMap;
use crate::vk::command_pool::CommandPool;
use crate::vk::logical_device::LogicalDevice;
use crate::vk::queue::QueueFamily;
use crate::vk::semaphore_pool::SemaphorePool;

pub struct FrameData {
    device : Arc<LogicalDevice>,
    pub index : usize,
    pub semaphore_pool : SemaphorePool,
    pub in_flight : vk::Fence,
    command_pools : IntMap<u32, CommandPool>,
}

impl FrameData {
    pub fn new(index : usize, device : &Arc<LogicalDevice>) -> Self {
        Self {
            device : device.clone(),
            index,
            in_flight : device.create_fence(vk::FenceCreateFlags::SIGNALED),
            semaphore_pool : SemaphorePool::new(device),
            command_pools : IntMap::default(),
        }
    }

    pub fn reset_command_pool(&mut self, family : &QueueFamily) {
        self.command_pools.entry(family.index())
            .and_modify(|pool| pool.reset(vk::CommandPoolResetFlags::default()));
    }

    pub fn get_command_buffer(&mut self, family : &QueueFamily, level : vk::CommandBufferLevel, count : u32) -> Vec<vk::CommandBuffer> {
        let pool = self.command_pools.entry(family.index())
            .or_insert(CommandPool::create(family, &self.device));

        pool.rent(level, count)
    }
}

impl Drop for FrameData {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_fence(self.in_flight, None);
        }
    }
}