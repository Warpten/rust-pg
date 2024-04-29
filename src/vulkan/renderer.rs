use std::{collections::HashSet, ffi::{self, c_char, CStr, CString}};

use ash::{khr, vk, Device, Entry};
use egui_winit::winit::{event_loop::EventLoop, window::Window};

use super::{Instance, PhysicalDevice, Surface};

pub struct Renderer<'a> {
    pub handle : ash::Entry,
    pub instance : Instance,
    pub surface : Surface,
    pub device : PhysicalDevice<'a>,
}

impl Renderer<'_> {
    fn create_entry() -> Entry {
        // If we link with vulkan, use linked(); use load().unwrap() otherwise
        Entry::linked()
    }

    pub fn new(width : u32, height : u32, extensions : Vec<CString>) -> Self {
        let entry = Entry::linked();
        let instance = Instance::new(&entry, CString::new("World Editor").unwrap(), &[CString::default(); 0]);
        let surface = Surface::new(&entry, &instance, todo!("fixme, add a window parameter!"));

        // Select a physical device
        // 1. GRAPHICS capable
        // 2. Able to present to a KHR swapchain
        // 3. With the requested extensions
        // 4. And swapchain capable.
        let physical_device = instance.get_physical_devices().iter().find(|physical_device| {
            // 1. Find a graphics capable queue family
            let graphics_family : Option<usize> = physical_device.queue_families.iter().find(|(i, family)| {
                family.properties.queue_flags.contains(vk::QueueFlags::GRAPHICS)
            }).map(|r| r.0).copied();

            // At this point `graphics_family` contains the index of the family capable of GRAPHICS.
            // If no family exists, this option is None.
            // To then check if this same `graphics_family` can present, we can just map that Option
            // and retrieve a boolean.

            // 2. Check if that same queue is capable of present
            let present_queue_capable = graphics_family.map(|family_index| {
                unsafe {
                    surface.loader.get_physical_device_surface_support(
                        physical_device.handle,
                        family_index as u32,
                        surface.handle
                    ).expect("Failed to get physical device surface support")
                }
            }).unwrap_or(false);

            // 3. Now, all that's left to do is check for extensions.
            // Normally, we would have a statically defined list of wanted extensions, but... alas.
            // We start by collecting a device's extensions and then remove them from the extensions
            // we asked for. If no extension subside, we're good.
            let mut device_extensions_names = physical_device.get_extensions().iter().map(|device_extension| {
                unsafe {
                    CStr::from_ptr(device_extension.extension_name.as_ptr()).to_owned()
                }
            }).collect::<Vec<_>>();

            let mut required_extensions = extensions.iter().map(|e| e.to_owned()).collect::<HashSet<_>>();
            for extension_name in device_extensions_names {
                required_extensions.remove(&extension_name);
            }
            let is_device_extension_supported = required_extensions.is_empty();

            // 4. Finally, check for swapchain support.
            let surface_formats = unsafe {
                surface.loader.get_physical_device_surface_formats(physical_device.handle, surface.handle)
                    .expect("Failed to get physical device surface formats")
            };

            let surface_present_modes = unsafe {
                surface.loader.get_physical_device_surface_present_modes(physical_device.handle, surface.handle)
                    .expect("Failed to get physical device surface present modes")
            };

            let is_swapchain_supported = !surface_formats.is_empty() && !surface_present_modes.is_empty();

            is_device_extension_supported && is_swapchain_supported && present_queue_capable && graphics_family.is_some()
        }).expect("Unable to select a physical device");

        Self {
            handle : entry,
            instance,
            surface,
            device : physical_device.clone(),
        }
    }
}