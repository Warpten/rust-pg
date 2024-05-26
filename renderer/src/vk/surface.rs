use std::sync::Arc;

use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::make_handle;
use crate::traits::handle::Handle;
use crate::vk::context::Context;
use crate::window::Window;

pub struct Surface {
    context : Arc<Context>,
    handle : vk::SurfaceKHR,
    pub loader : ash::khr::surface::Instance,
    // pub format : vk::SurfaceFormatKHR,
    // pub resolution : vk::Extent2D,
}

make_handle! { Surface, vk::SurfaceKHR }

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_surface(self.handle, None);
        }
    }
}

impl Surface {
    pub fn context(&self) -> &Arc<Context> { &self.context }

    /// Creates a new instance of [`Surface`].
    ///
    /// # Arguments
    /// 
    /// * `context` - The main Vulkan [`Context`].
    /// * `window` - An object providing a display handle and a window handle.
    /// 
    /// # Panics
    ///
    /// * Panics if [`ash_window::create_surface`] fails.
    /// * Panics if [`HasDisplayHandle::display_handle`] fails.
    /// * Panics if [`HasWindowHandle::window_handle`] fails.
    pub fn new(
        context : &Arc<Context>, 
        window : &Window
    ) -> Arc<Self> {
        let loader = ash::khr::surface::Instance::new(context.entry(), context.handle());
        let surface = unsafe {
            ash_window::create_surface(
                context.entry(),
                context.handle(),
                window.handle().display_handle().unwrap().into(),
                window.handle().window_handle().unwrap().into(), None
            ).expect("Failed to create surface")
        };

        Arc::new(Self {
            handle : surface,
            loader,
            context : context.clone()
        })
    }
}