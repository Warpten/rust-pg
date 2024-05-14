use std::{cmp::Ordering, collections::{HashMap, HashSet}, ffi::{CStr, CString}, hint, mem::ManuallyDrop, path::PathBuf, sync::{Arc, Mutex}};

use gpu_allocator::{vulkan::{Allocator, AllocatorCreateDesc}, AllocationSizes, AllocatorDebugSettings};
use nohash_hasher::IntMap;

use crate::{graph::Graph, queue, traits::{BorrowHandle, Handle}, Context, LogicalDevice, PhysicalDevice, PipelinePool, Queue, QueueFamily, RenderPass, Surface, Swapchain, SwapchainOptions, Window};

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum DynamicState<T> {
    Fixed(T),
    #[default]
    Dynamic
}

impl From<f32> for DynamicState<f32> {
    fn from(value: f32) -> Self {
        DynamicState::Fixed(value)
    }
}

#[derive(Debug)]
pub struct RendererOptions {
    pub(in crate) line_width : DynamicState<f32>,
    pub(in crate) device_extensions : Vec<CString>,
    pub(in crate) instance_extensions : Vec<CString>,
    pub(in crate) resolution :[u32; 2],
    pub(in crate) get_queue_count : fn(&QueueFamily) -> u32,
    pub(in crate) get_pipeline_cache_file : fn() -> PathBuf,
}

impl RendererOptions {
    #[inline] pub fn line_width(mut self, line_width : impl Into<DynamicState<f32>>) -> Self {
        self.line_width = line_width.into();
        self
    }

    #[inline] pub fn device_extensions(mut self, extensions : Vec<CString>) -> Self {
        self.device_extensions = extensions;
        self
    }

    #[inline] pub fn instance_extensions(mut self, extensions : Vec<CString>) -> Self {
        self.instance_extensions = extensions;
        self
    }

    #[inline] pub fn resolution(mut self, resolution : [u32; 2]) -> Self {
        self.resolution = resolution;
        self
    }

    #[inline] pub fn queue_count(mut self, getter : fn(&QueueFamily) -> u32) -> Self {
        self.get_queue_count = getter;
        self
    }

    #[inline] pub fn pipeline_cache_file(mut self, getter : fn() -> PathBuf) -> Self {
        self.get_pipeline_cache_file = getter;
        self
    }
}

impl Default for RendererOptions {
    fn default() -> Self {
        Self {
            line_width: DynamicState::Fixed(1.0f32),
            device_extensions: vec![ash::khr::swapchain::NAME.to_owned()],
            instance_extensions: vec![],
            resolution : [1280, 720],
            get_queue_count : |&_| 1,
            get_pipeline_cache_file : || "pipelines.dat".into(),
        }
    }
}

impl SwapchainOptions for RendererOptions {
    fn select_surface_format(&self, format : &ash::vk::SurfaceFormatKHR) -> bool {
        format.format == ash::vk::Format::B8G8R8A8_SRGB && format.color_space == ash::vk::ColorSpaceKHR::SRGB_NONLINEAR
    }

    fn width(&self) -> u32 { self.resolution[0] }
    fn height(&self) -> u32 { self.resolution[1] }

    fn present_mode(&self) -> ash::vk::PresentModeKHR { ash::vk::PresentModeKHR::MAILBOX }
}

pub struct Renderer {
    context : Arc<Context>,
    logical_device : Arc<LogicalDevice>,
    pipeline_cache : Arc<PipelinePool>,
    surface : Arc<Surface>,
    swapchain : Arc<Swapchain>,
    allocator : ManuallyDrop<Arc<Mutex<Allocator>>>,

    graph : Graph,
}

impl Renderer {
    #[inline] pub fn context(&self) -> &Arc<Context> { &self.context }
    #[inline] pub fn logical_device(&self) -> &Arc<LogicalDevice> { &self.logical_device }
    #[inline] pub fn pipeline_cache(&self) -> &Arc<PipelinePool> { &self.pipeline_cache }
    #[inline] pub fn surface(&self) -> &Arc<Surface> { &self.surface }
    #[inline] pub fn swapchain(&self) -> &Arc<Swapchain> { &self.swapchain }

    pub fn new(settings : &RendererOptions, window : &Window) -> Self {
        let context = unsafe {
            let mut all_extensions = settings.instance_extensions.clone();
            all_extensions.extend(window.surface_extensions().iter().map(|&extension| CStr::from_ptr(extension).to_owned()));
            all_extensions.dedup();

            Context::new(CString::new("send-help").unwrap_unchecked(), all_extensions)
        };
        let surface = Surface::new(&context, &window);

        let (physical_device, graphics_queue, presentation_queue) = select(&context, &surface, &settings);

        let queue_families = { // Deduplicate the graphics and presentation queues.
            let mut queue_families_map = IntMap::<u32, QueueFamily>::default();
            queue_families_map.entry(graphics_queue.index()).or_insert(graphics_queue);
            queue_families_map.entry(presentation_queue.index()).or_insert(presentation_queue);

            queue_families_map.into_values().collect::<Vec<_>>()
        };

        let logical_device = physical_device.create_logical_device(
            &context,
            queue_families.iter()
                .map(|queue : &QueueFamily| ((settings.get_queue_count)(queue), queue))
                .collect::<Vec<_>>(),
            |_index, _family| 1.0_f32,
            &settings.device_extensions);

        let swapchain = Swapchain::new(
            &context,
            &logical_device,
            &surface,
            settings,
            queue_families
        );

        let allocator = Allocator::new(&AllocatorCreateDesc {
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
            pipeline_cache : Arc::new(PipelinePool::new(logical_device.clone(), (settings.get_pipeline_cache_file)())),
            logical_device,
            surface,
            swapchain,
            graph,
            allocator : ManuallyDrop::new(Arc::new(Mutex::new(allocator)))
        }
    }
}

/// Selects a [`PhysicalDevice`] and its associated graphics and presentation [`queue families`](QueueFamily).
/// 
/// Device selection is done according to its classification, with the following order:
/// 
/// 1. [`ash::vk::PhysicalDeviceType::DISCRETE_GPU`]
/// 2. [`ash::vk::PhysicalDeviceType::INTEGRATED_GPU`]
/// 3. [`ash::vk::PhysicalDeviceType::VIRTUAL_GPU`]
/// 4. [`ash::vk::PhysicalDeviceType::CPU`]
/// 5. [`ash::vk::PhysicalDeviceType::OTHER`]
/// 
/// If possible, the graphics and presentation queue families will be the same to reduce internal synchronization.
/// 
fn select(context : &Arc<Context>, surface : &Arc<Surface>, settings : &RendererOptions) -> (PhysicalDevice, QueueFamily, QueueFamily) {
    context.get_physical_devices(
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

            let mut required_extensions = settings.device_extensions.iter()
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
            if family.is_graphics() {
                graphics_queue = Some(family.clone());
            }

            if family.can_present(&surface, &device) {
                present_queue = Some(family.clone());
            }

            // Found a family that can do both, immediately return it.
            if graphics_queue.is_some() && present_queue.is_some() {
                return Some((device, graphics_queue.unwrap(), present_queue.unwrap()));
            }
        }

        match (graphics_queue, present_queue) {
            (Some(g), Some(p)) => Some((device, g, p)),
            _ => None
        }
    })
    .expect("Failed to select a physical device and an associated queue family")
}