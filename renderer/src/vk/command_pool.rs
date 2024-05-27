use std::sync::Arc;

use ash::vk;
use crate::make_handle;
use crate::traits::handle::Handle;
use crate::vk::logical_device::LogicalDevice;
use crate::vk::queue::QueueFamily;

pub struct CommandPool {
    handle : vk::CommandPool,
    device : Arc<LogicalDevice>,
    pub family : u32,
}

impl CommandPool {
    pub fn builder(family : &QueueFamily) -> CommandPoolBuilder {
        CommandPoolBuilder::default(family)
    }

    pub fn device(&self) -> &LogicalDevice { &self.device }

    /// Resets this command pool.
    /// 
    /// # Arguments
    /// 
    /// * `flags` - A bitmask controlling the reset operation.
    /// 
    /// # Description
    ///
    /// Resetting a command pool recycles all the resources from all the command buffers allocated from
    /// the command pool back to the command pool. All command buffers that have been allocated from the
    /// command pool are put in the initial state.
    /// 
    /// Any primary command buffer allocated from another VkCommandPool that is in the recording or executable
    /// state and has a secondary command buffer allocated from commandPool recorded into it, becomes invalid.
    pub fn reset(&self, flags : vk::CommandPoolResetFlags) {
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
    pub fn free_command_buffers(&self, command_buffers : Vec<vk::CommandBuffer>) {
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
    pub fn trim(&self, flags : vk::CommandPoolTrimFlags) {
        unsafe {
            self.device.handle().trim_command_pool(self.handle, flags);
        }
    }
}

pub struct CommandPoolBuilder {
    flags : vk::CommandPoolCreateFlags,
    family_index : u32,
}

impl CommandPoolBuilder {
    pub(in crate) fn default(family : &QueueFamily) -> Self {
        Self {
            flags : vk::CommandPoolCreateFlags::empty(),
            family_index : family.index()
        }
    }

    pub fn transient(mut self) -> Self {
        self.flags |= vk::CommandPoolCreateFlags::TRANSIENT;
        self
    }
    
    pub fn reset(mut self) -> Self {
        self.flags |= vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER;
        self
    }
    
    pub fn protected(mut self) -> Self {
        self.flags |= vk::CommandPoolCreateFlags::PROTECTED;
        self
    }

    pub fn build(self, device : &Arc<LogicalDevice>) -> CommandPool {
        let handle = {
            let command_pool_create_info = vk::CommandPoolCreateInfo::default()
                .flags(self.flags)
                .queue_family_index(self.family_index);

            unsafe {
                device.handle().create_command_pool(&command_pool_create_info, None)
                    .expect("Failed to create command pool")
            }
        };

        CommandPool { handle, device : device.clone(), family : self.family_index }
    }
}

make_handle! { CommandPool, vk::CommandPool }

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_command_pool(self.handle, None)
        };
    }
}