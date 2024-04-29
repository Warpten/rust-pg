use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use super::Instance;

pub struct Surface<'instance> {
    pub instance : &'instance Instance,
    pub handle : vk::SurfaceKHR,
    pub loader : ash::khr::surface::Instance,
    // pub format : vk::SurfaceFormatKHR,
    // pub resolution : vk::Extent2D,
}

impl Drop for Surface<'_> {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_surface(self.handle, None);
        }
    }
}

impl<'instance> Surface<'instance> {
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
    /// Panics if creating the surface fails.
    pub fn new<T : HasDisplayHandle + HasWindowHandle>(
        entry : &ash::Entry,
        instance : &'instance Instance, 
        window : &T
    ) -> Self {
        let loader = ash::khr::surface::Instance::new(&entry, &instance.handle);
        let surface = unsafe {
            ash_window::create_surface(
                entry,
                &instance.handle,
                window.display_handle().unwrap().into(),
                window.window_handle().unwrap().into(), None)
                .expect("Failed to create surface")
        };

        Self { handle : surface, loader, instance }
    }
}