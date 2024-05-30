use egui_winit::winit;
use std::ffi::CString;

use crate::{gui::{
    renderer::{EguiCommand, ImageRegistry},
    run::ExitSignal,
}, vk::logical_device::LogicalDevice, window::Window};

/// egui theme type.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Theme {
    Light,
    Dark,
}

/// redraw handler type.
pub type RedrawHandler = Box<dyn FnOnce(winit::dpi::PhysicalSize<u32>, EguiCommand) + Send>;

/// return type of [`Application::request_redraw`].
pub enum HandleRedraw {
    Auto,
    Handle(RedrawHandler),
}

/// main egui-ash app trait.
pub trait Application {
    /// egui entry point
    fn ui(&mut self, ctx: &egui::Context);

    /// redraw the app.
    ///
    /// If you want to draw only egui, return [`HandleRedraw::Auto`].
    ///
    /// If you want to do your own Vulkan drawing in ash,
    /// return [`HandleRedraw::Handle(RedrawHandle)`] with FnOnce of drawing.
    /// NOTE: You must call `egui_cmd.update_swapchain` inside render function
    /// when you first render and when you recreate the swapchain.
    fn request_redraw(&mut self, _viewport_id: egui::ViewportId) -> HandleRedraw {
        HandleRedraw::Auto
    }
}

/// passed to [`AppCreator::create()`] for creating egui-ash app.
pub struct CreationContext<'a> {
    /// root window
    pub main_window: &'a Window,

    /// egui context
    pub context: egui::Context,

    /// required instance extensions for ash vulkan
    pub required_instance_extensions: Vec<CString>,

    /// required device extensions for ash vulkan
    pub required_device_extensions: Vec<CString>,

    /// user texture image registry for egui-ash
    pub image_registry: ImageRegistry,

    /// exit signal sender
    pub exit_signal: ExitSignal,
}

/// vulkan objects required for drawing ash.
/// You should return this struct from [`AppCreator::create()`].
pub struct AshRenderState {
    pub entry: Entry,
    pub instance: Instance,
    pub device: Arc<LogicalDevice>,
    pub surface: Surface,
    pub swapchain : Swapchain,
    pub queue : Queue,
    pub command_pool: CommandPool,
}

/// egui-ash app creator trait.
pub trait ApplicationCreator {
    type Application : Application;

    /// create egui-ash app.
    fn create(&self, cc: CreationContext) -> (Self::Application, AshRenderState);
}
