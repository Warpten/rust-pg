use std::sync::Arc;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::traits::handle::BorrowHandle;
use crate::traits::handle::Handle;
use crate::vk::Window;
use crate::vk::Context;

pub struct Surface {
    context : Arc<Context>,
    handle : ash::vk::SurfaceKHR,
    pub loader : ash::khr::surface::Instance,
    // pub format : vk::SurfaceFormatKHR,
    // pub resolution : vk::Extent2D,
}

impl Handle for Surface {
    type Target = ash::vk::SurfaceKHR;

    fn handle(&self) -> Self::Target { self.handle }
}

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