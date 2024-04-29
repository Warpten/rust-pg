use super::LogicalDevice;

pub struct CommandPool<'device, 'instance> {
    pub handle : ash::vk::CommandPool,
    pub device : &'device LogicalDevice<'device, 'instance>,
}

impl Drop for CommandPool<'_, '_> {
    fn drop(&mut self) {
        unsafe {
            self.device.handle.destroy_command_pool(self.handle, None)
        };
    }
}