use egui_winit::winit::{self, event_loop::EventLoop, window::WindowBuilder};
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};

use crate::ApplicationOptions;

pub struct Window {
    handle : winit::window::Window,
}

impl Window {
    pub fn new(
        options : &ApplicationOptions,
        event_loop : &EventLoop<()>
    ) -> Self {
        Self {
            handle : WindowBuilder::default()
                .with_title(options.title)
                .with_inner_size(winit::dpi::LogicalSize::new(options.resolution[0], options.resolution[1]))
                .build(event_loop)
                .expect("Window creation failed")
        }
    }

    pub fn surface_extensions(&self) -> Vec<*const i8> {
        let raw_display_handle : RawDisplayHandle = self.handle().display_handle()
            .map(Into::into)
            .expect("Unable to retrieve display handle");

        let mut surface_extension_names : Vec<*const i8> = ash_window::enumerate_required_extensions(raw_display_handle)
            .expect("Failed to enumerate required display extensions")
            .to_vec();

        surface_extension_names.push(ash::ext::debug_utils::NAME.as_ptr());
        surface_extension_names
    }

    pub fn handle(&self) -> &winit::window::Window { &self.handle }
    pub fn set_title(&mut self, title : &str) {
        self.handle.set_title(title)
    }

    pub fn size(&self) -> ash::vk::Extent2D {
        let size = self.handle.inner_size();
        ash::vk::Extent2D { width : size.width, height : size.height }
    }

    pub fn width(&self) -> u32 { self.handle.inner_size().width }
    pub fn height(&self) -> u32 { self.handle.inner_size().height }

    pub fn is_minimized(&self) -> bool {
        let size = self.handle.inner_size();
        size.width == 0 && size.height == 0
    }
}