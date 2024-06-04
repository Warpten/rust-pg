use ash::vk;
use egui_winit::winit::{self, event_loop::EventLoop, window::WindowBuilder};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle};

use crate::{application::ApplicationOptions, traits::handle::Handle, vk::{context::Context, physical_device::PhysicalDevice, queue::QueueFamily}};

pub struct Window {
    handle : winit::window::Window,

    surface : Option<(ash::khr::surface::Instance, vk::SurfaceKHR)>,
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        self.handle.display_handle()
    }
}

impl Window {
    pub fn new(
        options : &ApplicationOptions,
        event_loop : &EventLoop<()>
    ) -> Self {
        Self {
            handle : WindowBuilder::default()
                .with_title(options.title.clone())
                .with_inner_size(winit::dpi::LogicalSize::new(options.renderer_options.resolution[0], options.renderer_options.resolution[1]))
                .build(event_loop)
                .expect("Window creation failed"),
            surface : None
        }
    }

    pub(in crate) fn create_surface(&mut self, context : &Context) {
        unsafe {
            let surface_loader = ash::khr::surface::Instance::new(&context.entry, &context.instance);
            self.surface = match (self.handle.display_handle(), self.handle.window_handle()) {
                (Ok(display_handle), Ok(window_handle)) => {
                    let surface = ash_window::create_surface(&context.entry, &context.instance,
                        display_handle.as_raw(),
                        window_handle.as_raw(),
                        None
                    ).expect("Surface creation failed");

                    Some((surface_loader, surface))
                },
                _ => None,
            };
        }
    }

    pub fn get_surface_formats(&self, device : &PhysicalDevice) -> Vec<vk::SurfaceFormatKHR> {
        unsafe {
            if let Some(loader) = &self.surface {
                loader.0.get_physical_device_surface_formats(device.handle(), loader.1)
                    .expect("Failed to retrieve surface formats")
            } else {
                vec![]
            }
        }
    }

    pub fn get_surface_capabilities(&self, device : &PhysicalDevice) -> vk::SurfaceCapabilitiesKHR {
        unsafe {
            if let Some(loader) = &self.surface {
                loader.0.get_physical_device_surface_capabilities(device.handle(), loader.1)
                    .expect("Failed to retrieve surface capabilities")
            } else {
                vk::SurfaceCapabilitiesKHR::default()
            }
        }
    }

    pub fn get_surface_support(&self, device : &PhysicalDevice, queue : &QueueFamily) -> bool {
        unsafe {
            if let Some(loader) = &self.surface {
                loader.0.get_physical_device_surface_support(device.handle(), queue.index(), loader.1)
                    .expect("Failed to retrieve surface support")
            } else {
                false
            }
        }
    }

    pub fn get_present_modes(&self, device : &PhysicalDevice) -> Vec<vk::PresentModeKHR> {
        unsafe {
            if let Some(loader) = &self.surface {
                loader.0.get_physical_device_surface_present_modes(device.handle(), loader.1)
                    .expect("Failed to retrieve surface present modes")
            } else {
                vec![]
            }
        }
    }

    pub fn pixel_per_point(&self) -> f32 {
        self.handle.scale_factor() as _
    }

    pub fn surface_extensions(&self) -> Vec<*const i8> {
        let raw_display_handle : RawDisplayHandle = self.handle().display_handle()
            .map(Into::into)
            .expect("Unable to retrieve display handle");

        ash_window::enumerate_required_extensions(raw_display_handle)
            .expect("Failed to enumerate required display extensions")
            .to_vec()
    }

    pub fn surface(&self) -> vk::SurfaceKHR {
        match &self.surface {
            Some(surface) => surface.1,
            None => vk::SurfaceKHR::null()
        }
    }
    pub fn handle(&self) -> &winit::window::Window { &self.handle }

    pub fn set_title(&mut self, title : &str) {
        self.handle.set_title(title)
    }

    pub fn size(&self) -> vk::Extent2D {
        let size = self.handle.inner_size();
        vk::Extent2D { width : size.width, height : size.height }
    }

    pub fn width(&self) -> u32 { self.handle.inner_size().width }
    pub fn height(&self) -> u32 { self.handle.inner_size().height }

    pub fn is_minimized(&self) -> bool {
        let size = self.handle.inner_size();
        size.width == 0 && size.height == 0
    }
}