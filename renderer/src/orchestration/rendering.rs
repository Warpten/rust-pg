use std::sync::Arc;

use crate::vk::context::Context;
use crate::vk::frame_data::FrameData;
use crate::vk::logical_device::LogicalDevice;
use crate::vk::queue::QueueFamily;
use crate::vk::renderer::RendererOptions;
use crate::vk::swapchain::Swapchain;
use crate::window::Window;

/// A renderer is effectively a type that declares the need to work with its own render pass.
pub trait Renderable {
    /// Returns a recorded command buffer that contains all the commands needed to render the contents of this renderer.
    /// 
    /// # Arguments
    /// 
    /// * `swapchain` - The swapchain currently in use.
    /// * `frame_data` - A frame-specific data structure.
    fn record_commands(&mut self, swapchain : &Swapchain, frame_data : &FrameData);
    
    /// Returns an array of compatible framebuffers for this renderer.
    /// 
    /// # Arguments
    /// 
    /// * `swapchain` - The swapchain currently in use.
    fn create_framebuffers(&mut self, swapchain : &Swapchain);

    /// Returns a debug marker used with [`ash::vk::DebugUtilsLabelEXT`].
    fn marker_data<'a>(&self) -> (&'a str, [f32; 4]);
}

pub struct RenderingContextImpl {
    pub(in crate) context : Arc<Context>,
    pub device : LogicalDevice,
    pub window : Window,

    pub graphics_queue : QueueFamily,
    pub presentation_queue : QueueFamily,
    pub transfer_queue : QueueFamily,

    pub options : RendererOptions,
}
pub type RenderingContext = Arc<RenderingContextImpl>;

pub type RendererFn = fn(context : &RenderingContext, swapchain : &Swapchain) -> Box<dyn Renderable>;
