use egui_winit::winit::{self, event_loop::EventLoop, window::WindowBuilder};

pub struct Window {
    handle : winit::window::Window,
}

impl Window {
    pub fn new<Source : Into<String>>(
        width : u32,
        height : u32,
        title : Source,
        event_loop : &EventLoop<()>
    ) -> Self {
        let window = WindowBuilder::default()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(width, height))
            .build(event_loop)
            .expect("Failed to create window");

        Self {
            handle : window
        }
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

    pub fn is_minized(&self) -> bool {
        let size = self.handle.inner_size();
        size.width == 0 && size.height == 0
    }
}