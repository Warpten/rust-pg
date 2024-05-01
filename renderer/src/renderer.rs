use std::{collections::HashSet, ffi::{CStr, CString}, mem::ManuallyDrop, sync::{Arc, Mutex}};

use gpu_allocator::{vulkan::{Allocator, AllocatorCreateDesc}, AllocationSizes, AllocatorDebugSettings};

use crate::{traits::{BorrowHandle, Handle}, Instance, LogicalDevice, PhysicalDevice, QueueFamily, Surface, Swapchain, SwapchainOptions, Window};

pub struct Renderer<'a> {
    pub entry : Arc<ash::Entry>,
    pub instance : Arc<Instance>,
    pub logical_device : Arc<LogicalDevice>,
    pub surface : Arc<Surface>,
    pub swapchain : Arc<Swapchain>,
    pub window : &'a Window,
    pub allocator : ManuallyDrop<Arc<Mutex<Allocator>>>,
}

impl<'a> Renderer<'a> {
    pub fn new<T : SwapchainOptions>(window : &'a Window, instance_extensions : Vec<CString>, device_extensions : Vec<CString>, options : T) -> Self {
        let entry = Arc::new(ash::Entry::linked());
        let instance = unsafe {
            // TODO: This could probably use some cleaning up.
            let mut all_extensions = instance_extensions;
            for extension in window.surface_extensions() {
                all_extensions.push(CStr::from_ptr(extension).to_owned());
            }

            Instance::new(&entry, CString::new("World Editor").unwrap(), all_extensions)
        };
        let surface = Surface::new(&entry, instance.clone(), window);
        
        // Select a physical device
        // 1. GRAPHICS capable
        // 2. Able to present to a KHR swapchain
        // 3. With the requested extensions
        // 4. And swapchain capable.
        let (physical_device, graphics_queue, presentation_queue) = instance.get_physical_devices(
                |left, right| {
                    // TODO: Revisit this; DISCRETE_GPU > INTEGRATED_GPU > VIRTUAL_GPU > CPU > OTHER
                    left.properties.device_type.cmp(&right.properties.device_type)
                }
            )
            .into_iter()
            .filter(|device| -> bool {
                // 1. First, check for device extensions.
                // We start by collecting a device's extensions and then remove them from the extensions
                // we asked for. If no extension subside, we're good.
                let extensions_supported = {
                    let device_extensions_names = device.get_extensions().into_iter()
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
                        surface.loader.get_physical_device_surface_formats(device.handle(), surface.handle())
                            .expect("Failed to get physical device surface formats")
                    };

                    let surface_present_modes = unsafe {
                        surface.loader.get_physical_device_surface_present_modes(device.handle(), surface.handle())
                            .expect("Failed to get physical device surface present modes")
                    };

                    !surface_formats.is_empty() && !surface_present_modes.is_empty()
                };

                return extensions_supported && supports_present
            }).find_map(|device| -> Option<(PhysicalDevice, QueueFamily, QueueFamily)> {
                // At this point, the current device is eligible and we just need to check for a present queue and a graphics queue.
                // To do that, we will grab the queue's families.

                let mut graphics_queue = None;
                let mut present_queue = None;

                for family in &device.queue_families[..] {
                    if family.properties.queue_flags.contains(ash::vk::QueueFlags::GRAPHICS) {
                        graphics_queue = Some(family.clone());
                    }

                    if family.can_present(&surface, &device) {
                        present_queue = Some(family.clone());
                    }
                }

                match (graphics_queue, present_queue) {
                    (Some(g), Some(p)) => Some((device, g, p)),
                    _ => None
                }
            })
            .expect("Failed to select a physical device and an associated queue family");

        let logical_device = physical_device.create_logical_device(
            instance.clone(),
            vec![(1, graphics_queue), (1, presentation_queue)],
            |_index, _family| 1.0_f32,
            device_extensions);

        let swapchain = Swapchain::new(
            instance.clone(),
            logical_device.clone(),
            surface.clone(),
            options
        );

        let allocator = Allocator::new(&AllocatorCreateDesc{
            instance: instance.handle().clone(),
            device: logical_device.handle().clone(),
            physical_device: physical_device.handle().clone(),

            // TODO: All these may need tweaking and fixing
            debug_settings: AllocatorDebugSettings::default(),
            allocation_sizes : AllocationSizes::default(),
            buffer_device_address: false,
        }).unwrap();

        Self {
            entry,
            instance,
            logical_device,
            surface,
            swapchain,
            window,
            allocator : ManuallyDrop::new(Arc::new(Mutex::new(allocator)))
        }
    }
}