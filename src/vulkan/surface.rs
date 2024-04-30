use std::{ops::Deref, sync::Arc};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use super::Instance;

pub struct Surface {
    pub instance : Arc<Instance>,
    pub handle : ash::vk::SurfaceKHR,
    pub loader : ash::khr::surface::Instance,
    // pub format : vk::SurfaceFormatKHR,
    // pub resolution : vk::Extent2D,
}

impl Deref for Surface {
    type Target = ash::khr::surface::Instance;

    fn deref(&self) -> &Self::Target {
        &self.loader
    }
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
    /// * `instance` - The main vulkan objet.
    /// * `window` - An object providing a display handle and a window handle.
    /// 
    /// # Panics
    ///
    /// * Panics if [`ash_window::create_surface`] fails.
    /// * Panics if [`HasDisplayHandle::display_handle`] fails.
    /// * Panics if [`HasWindowHandle::window_handle`] fails.
    pub fn new<T : HasDisplayHandle + HasWindowHandle>(
        entry : Arc<ash::Entry>,
        instance : Arc<Instance>, 
        window : &T
    ) -> Arc<Self> {
        let loader = ash::khr::surface::Instance::new(&entry, &instance.handle);
        let surface = unsafe {
            ash_window::create_surface(
                &*entry,
                &instance.handle,
                window.display_handle().unwrap().into(),
                window.window_handle().unwrap().into(), None)
                .expect("Failed to create surface")
        };

        Arc::new(Self { handle : surface, loader, instance })
    }
}