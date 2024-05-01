use std::sync::Arc;

use crate::traits::{BorrowHandle, Handle};

use super::LogicalDevice;

pub struct CommandPool {
    pub(in super) handle : ash::vk::CommandPool,
    pub device : Arc<LogicalDevice>,
}

impl CommandPool {
    /// Resets this command pool.
    /// 
    /// # Arguments
    /// 
    /// * `flags` - A bitmask controlling the reset operation.
    /// 
    /// # Description
    /// Resetting a command pool recycles all of the resources from all of the command buffers allocated from
    /// the command pool back to the command pool. All command buffers that have been allocated from the
    /// command pool are put in the initial state.
    /// 
    /// Any primary command buffer allocated from another VkCommandPool that is in the recording or executable
    /// state and has a secondary command buffer allocated from commandPool recorded into it, becomes invalid.
    pub fn reset(&self, flags : ash::vk::CommandPoolResetFlags) {
        unsafe {
            let _ = self.device.handle().reset_command_pool(self.handle, flags);
        }
    }

    /// Frees a set of command buffers.
    /// 
    /// # Arguments
    /// 
    /// * `command_buffers` - A set of command buffers to be freed.
    /// 
    /// # Description
    /// 
    /// Any primary command buffer that is in the recording or executable state and has any element, of any of the
    /// given command buffers, recorded into it, becomes invalid.
    pub fn free_command_buffers(&self, command_buffers : Vec<ash::vk::CommandBuffer>) {
        unsafe {
            self.device.handle().free_command_buffers(self.handle, &command_buffers);
        }
    }

    /// Trims the command pool, recycling unused memory back to the system. Command buffers allocated from the pool
    /// are not affected.
    /// 
    /// This is a somewhat expensive operation; if don't know what you're doing, don't use it.
    /// 
    /// # Arguments
    /// 
    /// * `flags` - Reserved for future uses.
    pub fn trim(&self, flags : ash::vk::CommandPoolTrimFlags) {
        unsafe {
            self.device.handle().trim_command_pool(self.handle, flags);
        }
    }
}

impl Handle for CommandPool {
    type Target = ash::vk::CommandPool;

    fn handle(&self) -> ash::vk::CommandPool { self.handle }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_command_pool(self.handle, None)
        };
    }
}