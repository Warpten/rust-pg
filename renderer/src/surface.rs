use std::sync::Arc;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::traits::BorrowHandle;
use crate::traits::Handle;
use crate::Window;
use crate::Instance;

pub struct Surface {
    pub instance : Arc<Instance>,
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
    /// Creates a new instance of [`Surface`].
    ///
    /// # Arguments
    /// 
    /// * `entry` - Holds Vulkan functions independant of Device or Instance.
    /// * `instance` - The main Vulkan [`Instance`].
    /// * `window` - An object providing a display handle and a window handle.
    /// 
    /// # Panics
    ///
    /// * Panics if [`ash_window::create_surface`] fails.
    /// * Panics if [`HasDisplayHandle::display_handle`] fails.
    /// * Panics if [`HasWindowHandle::window_handle`] fails.
    pub fn new(
        entry : &Arc<ash::Entry>,
        instance : Arc<Instance>, 
        window : &Window
    ) -> Arc<Self> {
        let loader = ash::khr::surface::Instance::new(&entry, instance.handle());
        let surface = unsafe {
            ash_window::create_surface(
                &*entry,
                instance.handle(),
                window.handle().display_handle().unwrap().into(),
                window.handle().window_handle().unwrap().into(), None
            ).expect("Failed to create surface")
        };

        Arc::new(Self { handle : surface, loader, instance })
    }
}