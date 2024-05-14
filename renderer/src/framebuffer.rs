use std::sync::Arc;

use crate::{traits::{BorrowHandle, Handle}, LogicalDevice, RenderPass};

// This whole file needs cleaning
// - Device should be Arc<LogicalDevice> and stored
// - This type should probably implement Drop
// - views should own, but it doens't (this is probably a leak!)

pub struct Framebuffer {
    handle : ash::vk::Framebuffer,
    views : Vec<ash::vk::ImageView>,
}

impl Framebuffer {
    pub fn new(extent : ash::vk::Extent2D, image_views : &[ash::vk::ImageView], layers : u32, device : &Arc<LogicalDevice>, render_pass : &Arc<RenderPass>) -> Framebuffer {
        let framebuffer_create_info = ash::vk::FramebufferCreateInfo::default()
            .height(extent.height)
            .width(extent.width)
            .attachments(image_views)
            .render_pass(render_pass.handle())
            .layers(layers);

        let handle = unsafe {
            device.handle().create_framebuffer(&framebuffer_create_info, None)
                .expect("Creating the framebuffer failed")
        };

        Self { handle, views : image_views.to_vec() }
    }

    pub fn views(&self) -> &Vec<ash::vk::ImageView> { &self.views }
}

impl Handle for Framebuffer {
    type Target = ash::vk::Framebuffer;

    fn handle(&self) -> Self::Target { self.handle }
}
