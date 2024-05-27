use std::sync::Arc;

use ash::vk;
use crate::make_handle;
use crate::vk::logical_device::LogicalDevice;

// This whole file needs cleaning
// - Device should be Arc<LogicalDevice> and stored
// - This type should probably implement Drop
// - views should own, but it doens't (this is probably a leak!)

pub struct Framebuffer {
    device : Arc<LogicalDevice>,
    handle : vk::Framebuffer,
}

impl Framebuffer {
    pub fn new(device : &Arc<LogicalDevice>, create_info : vk::FramebufferCreateInfo) -> Framebuffer {
        let handle = unsafe {
            device.handle().create_framebuffer(&create_info, None)
                .expect("Creating the framebuffer failed")
        };

        Self { handle, device : device.clone() }
    }
}

make_handle! { Framebuffer, vk::Framebuffer }

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_framebuffer(self.handle, None);
        }
    }
}