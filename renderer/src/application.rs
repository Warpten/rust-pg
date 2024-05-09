use std::{cmp::Ordering, collections::HashSet, ffi::{CStr, CString}, hint, mem::ManuallyDrop, sync::{Arc, Mutex}};

use gpu_allocator::{vulkan::{Allocator, AllocatorCreateDesc}, AllocationSizes, AllocatorDebugSettings};

use crate::{graph::{pass::Pass, texture::Texture, Graph}, traits::{BorrowHandle, Handle}, Context, LogicalDevice, PhysicalDevice, QueueFamily, Surface, Swapchain, SwapchainOptions, Window};

pub struct Application<'a> {
    context : Arc<Context>,
    logical_device : Arc<LogicalDevice>,
    surface : Arc<Surface>,
    swapchain : Arc<Swapchain>,
    window : &'a Window,
    allocator : ManuallyDrop<Arc<Mutex<Allocator>>>,

    graph : Graph,
}

impl<'a> Application<'a> {
    pub fn new<T : SwapchainOptions>(window : &'a Window, instance_extensions : Vec<CString>, device_extensions : Vec<CString>, options : T) -> Self {
        let context = unsafe {
            // TODO: This could probably use some cleaning up.
            let mut all_extensions = instance_extensions;
            for extension in window.surface_extensions() {
                all_extensions.push(CStr::from_ptr(extension).to_owned());
            }

            Context::new(CString::new("World Editor").unwrap(), all_extensions)
        };
        let surface = Surface::new(context.clone(), window);
        
        // Select a physical device
        // 1. GRAPHICS capable
        // 2. Able to present to a KHR swapchain
        // 3. With the requested extensions
        // 4. And swapchain capable.
        let (physical_device, graphics_queue, presentation_queue) = context.get_physical_devices(
                |left, right| {
                    // DISCRETE_GPU > INTEGRATED_GPU > VIRTUAL_GPU > CPU > OTHER
                    match (right.properties().device_type, left.properties().device_type) {
                        // Base equality case
                        (a, b) if a == b => Ordering::Equal,

                        // DISCRETE_GPU > ALL
                        (ash::vk::PhysicalDeviceType::DISCRETE_GPU, _) => Ordering::Greater,

                        // DISCRETE > INTEGRATED > ALL
                        (ash::vk::PhysicalDeviceType::INTEGRATED_GPU, ash::vk::PhysicalDeviceType::DISCRETE_GPU) => Ordering::Less,
                        (ash::vk::PhysicalDeviceType::INTEGRATED_GPU, _) => Ordering::Greater,

                        // DISCRETE, INTEGRATED > VIRTUAL > ALL
                        (ash::vk::PhysicalDeviceType::VIRTUAL_GPU, ash::vk::PhysicalDeviceType::DISCRETE_GPU) => Ordering::Less,
                        (ash::vk::PhysicalDeviceType::VIRTUAL_GPU, ash::vk::PhysicalDeviceType::INTEGRATED_GPU) => Ordering::Less,
                        (ash::vk::PhysicalDeviceType::VIRTUAL_GPU, _) => Ordering::Greater,

                        // DISCRETE, INTEGRATED, VIRTUAL > CPU > ALL
                        (ash::vk::PhysicalDeviceType::CPU, ash::vk::PhysicalDeviceType::DISCRETE_GPU) => Ordering::Less,
                        (ash::vk::PhysicalDeviceType::CPU, ash::vk::PhysicalDeviceType::INTEGRATED_GPU) => Ordering::Less,
                        (ash::vk::PhysicalDeviceType::CPU, ash::vk::PhysicalDeviceType::VIRTUAL_GPU) => Ordering::Less,
                        (ash::vk::PhysicalDeviceType::CPU, _) => Ordering::Greater,

                        // ALL > OTHER
                        (ash::vk::PhysicalDeviceType::OTHER, _) => Ordering::Less,

                        // Default case for branch solver
                        (_, _) => unsafe { hint::unreachable_unchecked() },
                    }
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
            context.clone(),
            vec![(1, graphics_queue), (1, presentation_queue)],
            |_index, _family| 1.0_f32,
            device_extensions);

        let swapchain = Swapchain::new(
            context.clone(),
            logical_device.clone(),
            surface.clone(),
            options
        );

        let allocator = Allocator::new(&AllocatorCreateDesc{
            instance: context.handle().clone(),
            device: logical_device.handle().clone(),
            physical_device: physical_device.handle().clone(),

            // TODO: All these may need tweaking and fixing
            debug_settings: AllocatorDebugSettings::default(),
            allocation_sizes : AllocationSizes::default(),
            buffer_device_address: false,
        }).unwrap();

        let graph = Graph::new();

        Self {
            context,
            logical_device,
            surface,
            swapchain,
            window,
            graph,
            allocator : ManuallyDrop::new(Arc::new(Mutex::new(allocator)))
        }
    }

    #[inline] pub fn context(&self) -> &Arc<Context> { &self.context }
    #[inline] pub fn entry(&self) -> &Arc<ash::Entry> { self.context().entry() }
    #[inline] pub fn logical_device(&self) -> &Arc<LogicalDevice> { &self.logical_device }
    #[inline] pub fn surface(&self) -> &Arc<Surface> { &self.surface }
    #[inline] pub fn swapchain(&self) -> &Arc<Swapchain> { &self.swapchain }
    #[inline] pub fn window(&self) -> &'a Window { &self.window }
    #[inline] pub fn allocator(&self) -> &Arc<Mutex<Allocator>> { &self.allocator }

    pub fn on_swapchain_created(&mut self) {
        self.graph.reset();

        // Obviously not actual code, scaffolding tests
        // (making sure stuff compiles)

        let backbuffer = Texture::new("builtin://backbuffer", 1, 1, ash::vk::Format::A8B8G8R8_UINT_PACK32)
            .register(&mut self.graph);

        let a = Pass::new("Pass A")
            .add_output("Backbuffer output", backbuffer.into())
            .register(&mut self.graph);

        let b = Pass::new("Pass B")
            .add_input("Backbuffer input", a.output(&self.graph, "Backbuffer output"))
            .register(&mut self.graph);

        self.graph.build();
    }
}