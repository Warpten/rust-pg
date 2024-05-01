use std::ffi::CString;

use egui_winit::winit;

use crate::exit_signal::ExitSignal;

use super::ImageRegistry;

pub struct CreationContext<'a> {
    /// root window
    pub main_window: &'a winit::window::Window,

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