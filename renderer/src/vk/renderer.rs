use std::path::PathBuf;

use ash::vk;

use super::{queue::QueueFamily, swapchain::SwapchainOptions};

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy, Clone)]
pub enum DynamicState<T> {
    Fixed(T),
    #[default]
    Dynamic
}

impl From<f32> for DynamicState<f32> {
    fn from(value: f32) -> Self {
        DynamicState::Fixed(value)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RendererOptions {
    pub(in crate) line_width : DynamicState<f32>,
    pub(in crate) get_queue_count : fn(&QueueFamily) -> u32,
    pub(in crate) get_pipeline_cache_file : fn() -> PathBuf,
    pub(in crate) depth : bool,
    pub(in crate) stencil : bool,
    pub(in crate) separate_depth_stencil : bool, // NYI
    pub(in crate) clear_color : [f32; 4],
    pub multisampling : vk::SampleCountFlags,
}

impl RendererOptions {
    #[inline] pub fn line_width(mut self, line_width : impl Into<DynamicState<f32>>) -> Self {
        self.line_width = line_width.into();
        self
    }
    
    #[inline] pub fn queue_count(mut self, getter : fn(&QueueFamily) -> u32) -> Self {
        self.get_queue_count = getter;
        self
    }

    #[inline] pub fn pipeline_cache_file(mut self, getter : fn() -> PathBuf) -> Self {
        self.get_pipeline_cache_file = getter;
        self
    }

    value_builder! { depth, bool }
    value_builder! { stencil, bool }
    value_builder! { clear_color, [f32; 4] }
    value_builder! { multisampling, samples, multisampling, vk::SampleCountFlags }
}

impl Default for RendererOptions {
    fn default() -> Self {
        Self {
            line_width: DynamicState::Fixed(1.0f32),
            get_queue_count : |&_| 1,
            get_pipeline_cache_file : || "pipelines.dat".into(),
            depth : true,
            stencil : true,
            separate_depth_stencil : false,
            clear_color : [0.0f32, 0.0f32, 0.0f32, 0.0f32],
            multisampling : vk::SampleCountFlags::TYPE_1,
        }
    }
}

impl SwapchainOptions for RendererOptions {
    fn select_surface_format(&self, format : &vk::SurfaceFormatKHR) -> bool {
        format.format == vk::Format::B8G8R8A8_SRGB && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
    }

    fn depth(&self) -> bool { self.depth }
    fn stencil(&self) -> bool { self.stencil }
    fn multisampling(&self) -> vk::SampleCountFlags { self.multisampling }
}
