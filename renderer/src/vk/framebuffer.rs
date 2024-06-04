use ash::vk;
use crate::make_handle;
use crate::orchestration::rendering::RenderingContext;

// This whole file needs cleaning
// - views should own, but it doens't (this is probably a leak!)

pub struct Framebuffer {
    context : RenderingContext,
    handle : vk::Framebuffer,
}

impl Framebuffer {
    pub fn new(context : &RenderingContext, create_info : vk::FramebufferCreateInfo) -> Framebuffer {
        let handle = unsafe {
            context.device.handle().create_framebuffer(&create_info, None)
                .expect("Creating the framebuffer failed")
        };

        Self { handle, context : context.clone() }
    }
}

make_handle! { Framebuffer, vk::Framebuffer }

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            self.context.device.handle().destroy_framebuffer(self.handle, None);
        }
    }
}