use std::{collections::HashSet, ffi::{self, c_char, CStr, CString}};

use ash::{khr, vk, Device, Entry};
use egui_winit::winit::{event_loop::EventLoop, window::Window};

use crate::vulkan::SwapchainOptions;

use super::{Instance, PhysicalDevice, QueueFamily, Surface, Swapchain};

pub struct Renderer<'instance> {
    pub handle : ash::Entry,
    pub instance : Instance,
    pub surface : Surface<'instance>,
    pub device : PhysicalDevice<'instance>,
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

impl Renderer<'_> {
    fn create_entry() -> Entry {
        // If we link with vulkan, use linked(); use load().unwrap() otherwise
        Entry::linked()
    }

    /// Returns a graphics queue family capable of
    /// 1. GRAPHICS
    /// 2. Present
    /// 3. Has required extensions
    /// 4. Has swapchain support
    fn get_capable_queue<'device, 'instance>(
        physical_device : &'device PhysicalDevice<'instance>,
        surface : &'instance Surface<'instance>,
        extensions : Vec<CString>
    ) -> Option<&'device QueueFamily> {
        // 1. Collect all graphics enabled queue families.
        let mut graphics_family = physical_device.queue_families.iter()
            .filter(|&family| {
                family.properties.queue_flags.contains(vk::QueueFlags::GRAPHICS)
            });

        // 2. Filter again, restricting to GRAPHICS queue families that also support present.
        let mut graphics_family = graphics_family.find(|&family| {
            family.can_present(&surface, physical_device)
        });

        let mut graphics_family = graphics_family.filter(|_| {
            // 3. Now, all that's left to do is check for extensions.
            // Normally, we would have a statically defined list of wanted extensions, but... alas.
            // We start by collecting a device's extensions and then remove them from the extensions
            // we asked for. If no extension subside, we're good.
            let mut device_extensions_names = physical_device.get_extensions().iter()
                .map(|device_extension| {
                    unsafe {
                        CStr::from_ptr(device_extension.extension_name.as_ptr()).to_owned()
                    }
                }).collect::<Vec<_>>();

            let mut required_extensions = extensions.iter()
                .map(|e| e.to_owned())
                .collect::<HashSet<_>>();
            for extension_name in device_extensions_names {
                required_extensions.remove(&extension_name);
            }

            required_extensions.is_empty()
        });

        let mut graphics_family = graphics_family.filter(|_| {
            // 4. Finally, check for swapchain support.
            let surface_formats = unsafe {
                surface.loader.get_physical_device_surface_formats(physical_device.handle, surface.handle)
                    .expect("Failed to get physical device surface formats")
            };

            let surface_present_modes = unsafe {
                surface.loader.get_physical_device_surface_present_modes(physical_device.handle, surface.handle)
                    .expect("Failed to get physical device surface present modes")
            };

            !surface_formats.is_empty() && !surface_present_modes.is_empty()
        });

        graphics_family
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
        let (physical_device, queue_family) = instance.get_physical_devices(
                |&left, &right| {
                    // TODO: Revisit this; DISCRETE_GPU > INTEGRATED_GPU > VIRTUAL_GPU > CPU > OTHER
                    left.properties.device_type.cmp(&right.properties.device_type)
                }
            )
            .iter()
            .find_map(|&physical_device| -> Option<(&PhysicalDevice, &QueueFamily)> {
                Self::get_capable_queue(&physical_device, &surface, extensions)
                    .map(|queue| (&physical_device, queue))
            })
            .expect("Failed to select a physical device and an associated queue family");

        let logical_device = queue_family.create_logical_device(
            &instance, physical_device, 1, |_| 1.0_f32, vec![]);

        let swapchain_options = options::Swapchain {
            queue_families : vec![queue_family.clone()],
            width,
            height
        };

        let swapchain = Swapchain::new(
            &instance,
            &logical_device,
            &surface,
            swapchain_options);

        Self {
            handle : entry,
            instance,
            surface,
            device : physical_device.clone(),
        }
    }
}