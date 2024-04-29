use ash::vk;
use egui_winit::winit;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use super::Instance;

pub struct Surface {
    pub handle : vk::SurfaceKHR,
    pub loader : ash::khr::surface::Instance,
    // pub format : vk::SurfaceFormatKHR,
    // pub resolution : vk::Extent2D,
}

impl Surface {
    pub fn new(entry : &ash::Entry, instance : &Instance, window : &winit::window::Window) -> Self {
        let loader = ash::khr::surface::Instance::new(&entry, &instance.handle);
        let surface = unsafe {
            ash_window::create_surface(
                entry,
                &instance.handle,
                window.display_handle().unwrap().into(),
                window.window_handle().unwrap().into(), None)
                .expect("Failed to create surface")
        };

        Self { handle : surface, loader : loader }
    }
}