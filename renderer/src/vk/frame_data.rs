use std::sync::Arc;

use ash::vk;
use nohash_hasher::IntMap;
use crate::vk::{CommandPool, LogicalDevice, SemaphorePool};

use super::QueueFamily;

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

    pub fn get_command_buffer(&mut self, family : &QueueFamily, level : vk::CommandBufferLevel, count : u32) -> Vec<vk::CommandBuffer> {
        let pool = self.command_pools.entry(family.index())
            .or_insert(CommandPool::create(family, &self.device));

        pool.rent(level, count)
    }
}