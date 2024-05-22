use std::sync::Arc;

use crate::{traits::handle::{BorrowHandle, Handle}, vk::{LogicalDevice, RenderPass}};

// This whole file needs cleaning
// - Device should be Arc<LogicalDevice> and stored
// - This type should probably implement Drop
// - views should own, but it doens't (this is probably a leak!)

pub struct Framebuffer {
    handle : ash::vk::Framebuffer,
}

impl Framebuffer {
    pub fn new(device : &Arc<LogicalDevice>, create_info : ash::vk::FramebufferCreateInfo) -> Framebuffer {
        let handle = unsafe {
            device.handle().create_framebuffer(&create_info, None)
                .expect("Creating the framebuffer failed")
        };

        Self { handle }
    }
}

impl Handle for Framebuffer {
    type Target = ash::vk::Framebuffer;

    fn handle(&self) -> Self::Target { self.handle }
}
