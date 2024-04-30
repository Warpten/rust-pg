use std::{collections::HashSet, ffi::{self, c_char, CStr, CString}, sync::Arc};

use ash::{khr, vk, Device, Entry};
use egui_winit::winit::{event_loop::EventLoop, window::Window};

use crate::vulkan::SwapchainOptions;

use super::{Instance, PhysicalDevice, QueueFamily, Surface, Swapchain};

pub struct Renderer{
    pub handle : Arc<ash::Entry>,
    pub instance : Arc<Instance>,
    pub surface : Arc<Surface>,
    pub device : Arc<PhysicalDevice>,
}

mod options {
    use crate::vulkan::{QueueFamily, SwapchainOptions};

    pub struct Swapchain {
        pub queue_families : Vec<QueueFamily>,
        pub width : u32,
        pub height : u32
    }

    impl SwapchainOptions for Swapchain {
        fn select_surface_format(&self, format : &ash::vk::SurfaceFormatKHR) -> bool {
            format.format == ash::vk::Format::B8G8R8A8_UNORM || format.format == ash::vk::Format::R8G8B8A8_UNORM
        }
    
        fn queue_families(&self) -> Vec<QueueFamily> { self.queue_families }
    
        fn width(&self) -> u32 { self.width }

        fn height(&self) -> u32 { self.height }
        
        fn composite_alpha(&self) -> ash::vk::CompositeAlphaFlagsKHR { ash::vk::CompositeAlphaFlagsKHR::OPAQUE }

        fn present_mode(&self) -> ash::vk::PresentModeKHR { ash::vk::PresentModeKHR::FIFO }
    }
}

impl Renderer {
    /// Creates a new [`Renderer`].
    /// 
    /// # Arguments
    /// 
    /// * `width` - Width of the viewport
    /// * `height` - Height of the viewport
    /// * `instance_extensions` - A set of instance level extension names.
    /// * `device_extensions` - A set of device level extension names.
    ///
    /// # Panics
    ///
    /// Panics if a ton of stuff goes wrong. Really, don't look.
    pub fn new(width : u32, height : u32, instance_extensions : Vec<CString>, device_extensions : Vec<CString>) -> Self {
        let entry = Arc::new(Entry::linked());
        let instance = Instance::new(entry, CString::new("World Editor").unwrap(), instance_extensions);
        let surface = Surface::new(entry, instance, todo!("fixme, add a window parameter!"));

        // Select a physical device
        // 1. GRAPHICS capable
        // 2. Able to present to a KHR swapchain
        // 3. With the requested extensions
        // 4. And swapchain capable.
        let (physical_device, graphics_queue, presentation_queue) = instance.get_physical_devices(
                |&left, &right| {
                    // TODO: Revisit this; DISCRETE_GPU > INTEGRATED_GPU > VIRTUAL_GPU > CPU > OTHER
                    left.properties.device_type.cmp(&right.properties.device_type)
                }
            )
            .iter()
            .filter(|device| -> bool {
                // 1. First, check for device extensions.
                // We start by collecting a device's extensions and then remove them from the extensions
                // we asked for. If no extension subside, we're good.
                let extensions_supported = {
                    let mut device_extensions_names = device.get_extensions().iter()
                        .map(|device_extension| {
                            unsafe {
                                CStr::from_ptr(device_extension.extension_name.as_ptr()).to_owned()
                            }
                        }).collect::<Vec<_>>();

                    let mut required_extensions = device_extensions.iter()
                        .map(|e| e.to_owned())
                        .collect::<HashSet<_>>();
                    for extension_name in device_extensions_names {
                        required_extensions.remove(&extension_name);
                    }

                    required_extensions.is_empty()
                };

                // 2. Finally, check for swapchain support.
                let supports_present = {
                    let surface_formats = unsafe {
                        surface.loader.get_physical_device_surface_formats(device.handle, surface.handle)
                            .expect("Failed to get physical device surface formats")
                    };

                    let surface_present_modes = unsafe {
                        surface.loader.get_physical_device_surface_present_modes(device.handle, surface.handle)
                            .expect("Failed to get physical device surface present modes")
                    };

                    !surface_formats.is_empty() && !surface_present_modes.is_empty()
                };

                return extensions_supported && supports_present
            })
            .find_map(|&physical_device| -> Option<(&Arc<PhysicalDevice>, &QueueFamily, &QueueFamily)> {
                let graphics_queue = physical_device.queue_families.iter()
                    .find(|&family| {
                        family.properties.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                    });

                let present_queue = physical_device.queue_families.iter()
                    .find(|&family| {
                        family.can_present(surface, physical_device)
                    });

                let queues = graphics_queue.zip(present_queue);

                Some(&physical_device).and_then(|d| queues.map(|(g, p)| (d, g, p)))
            })
            .expect("Failed to select a physical device and an associated queue family");

        let logical_device = physical_device.create_logical_device(
            instance, vec![(1, graphics_queue), (1, presentation_queue)], |_index, _family| 1.0_f32, device_extensions);

        let swapchain_options = options::Swapchain {
            queue_families : vec![*graphics_queue, *presentation_queue],
            width,
            height
        };

        let swapchain = Swapchain::new(
            instance,
            logical_device,
            surface,
            swapchain_options);

        Self {
            handle : entry,
            instance,
            surface,
            device : physical_device.clone(),
        }
    }
}