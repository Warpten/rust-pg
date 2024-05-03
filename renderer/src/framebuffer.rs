use crate::Swapchain;

pub struct Framebuffer {
    handle : ash::vk::Framebuffer,
}

impl Framebuffer {
    pub fn new(extent : ash::vk::Extent2D, swapchain : &Swapchain) {
        let image_views = swapchain.images().map(|(image, view)| view).collect::<Vec<_>>();

        let framebuffer_create_info = ash::vk::FramebufferCreateInfo::default()
            .height(extent.height)
            .width(extent.width)
            .attachments(&image_views[..])
            .layers(swapchain.layer_count());
    }
}