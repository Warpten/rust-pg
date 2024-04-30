use std::{ops::Deref, sync::Arc};

use super::LogicalDevice;

pub struct CommandPool {
    pub handle : ash::vk::CommandPool,
    pub device : Arc<LogicalDevice>,
}

impl Deref for CommandPool {
    type Target = ash::vk::CommandPool;

    fn deref(&self) -> &Self::Target { &self.handle }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.handle.destroy_command_pool(self.handle, None)
        };
    }
}