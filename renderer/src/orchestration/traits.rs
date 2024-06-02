use std::sync::Arc;

use ash::vk;
use egui_winit::winit::event::WindowEvent;

use crate::{vk::{command_buffer::CommandBuffer, logical_device::LogicalDevice, pipeline::pool::PipelinePool, render_pass::RenderPassCreateInfo, swapchain::Swapchain}, window::Window};

use super::orchestrator::RenderingContext;

/// The counterpart to [`Renderable`] for initialization of Vulkan structures.
pub trait RenderableFactory {
    fn build(&self, context : &Arc<RenderingContext>) -> Box<dyn Renderable>;

    /// Mutates the render pass creation information to add a subpass.
    fn express_dependencies(&self, create_info : RenderPassCreateInfo) -> RenderPassCreateInfo;
}

/// Describes an object that needs to emit draw commands.
pub trait Renderable {
    fn handle_event(&mut self, event : &WindowEvent, window : &Window) -> bool {
        false
    }

    /// Adds draw commands for the current frame.
    /// 
    /// # Arguments
    /// 
    /// * `cmd` - The command buffer for which commands will be recorded.
    /// * `frame_index` - The index of the current frame.
    fn draw_frame(&mut self, cmd : &CommandBuffer, frame_index : usize);
    
    /// Specifies how the contents of this pass are recorded to a command buffer.
    fn contents_type(&self) -> vk::SubpassContents {
        vk::SubpassContents::INLINE
    }
}

pub type RenderableFactoryProvider = fn(&Arc<LogicalDevice>, &Arc<Swapchain>, &Arc<PipelinePool>) -> Box<dyn RenderableFactory>;
