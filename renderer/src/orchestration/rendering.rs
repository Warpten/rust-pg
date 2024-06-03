use std::cmp::Ordering;
use std::collections::HashSet;
use std::ffi::CStr;
use std::mem::ManuallyDrop;
use std::{hint, slice};
use std::sync::{Arc, Mutex};

use ash::vk;
use egui_winit::winit::event::WindowEvent;
use egui_winit::EventResponse;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use gpu_allocator::{AllocationSizes, AllocatorDebugSettings};
use nohash_hasher::IntMap;

use crate::application::RendererError;
use crate::traits::handle::Handle;
use crate::vk::context::Context;
use crate::vk::frame_data::FrameData;
use crate::vk::framebuffer::Framebuffer;
use crate::vk::logical_device::LogicalDevice;
use crate::vk::physical_device::PhysicalDevice;
use crate::vk::pipeline::pool::PipelinePool;
use crate::vk::queue::{QueueAffinity, QueueFamily};
use crate::vk::renderer::RendererOptions;
use crate::vk::surface::Surface;
use crate::vk::swapchain::Swapchain;
use crate::window::Window;

/// A renderer is effectively a type that declares the need to work with its own render pass.
pub trait Renderer {
    /// Returns a recorded command buffer that contains all the commands needed to render the contents of this renderer.
    fn record_commands(&mut self, framebuffer : &Framebuffer, frame_data : &FrameData);
    fn create_framebuffers(&mut self, swapchain : &Arc<Swapchain>) -> Vec<Framebuffer>;

    fn marker_label(&self) -> String;
    fn marker_color(&self) -> [f32; 4];

    fn handle_event(&mut self, event : &WindowEvent) -> EventResponse {
        EventResponse { repaint : false, consumed : false }
    }
}

pub struct RenderingContext {
    context : Arc<Context>,
    pub surface : Arc<Surface>,
    pub device : Arc<LogicalDevice>,
    pub graphics_queue : QueueFamily,
    pub presentation_queue : QueueFamily,
    pub transfer_queue : QueueFamily,
    pub swapchain : Arc<Swapchain>,
    allocator : ManuallyDrop<Arc<Mutex<Allocator>>>,
    pub pipeline_cache : Arc<PipelinePool>,
    pub window : Window,

    pub options : RendererOptions,
}

pub type RendererFn = fn(context : &Arc<RenderingContext>) -> Box<dyn Renderer>;

pub struct Orchestrator {
    context : Arc<Context>,
    renderers : Vec<RendererFn>,
}
impl Orchestrator {
    /// Creates a new orchestrator. This object is in charge of preparing Vulkan structures for rendering
    /// as well as the way command buffers will be recorded and executed.
    pub fn new(context : &Arc<Context>) -> Self {
        Self {
            context : context.clone(),
            renderers : vec![],
        }
    }

    /// Adds a renderable to this orchestrator. See the documentation on [`Renderer`] for more informations.
    pub fn add_renderer(mut self, renderer : RendererFn) -> Self {
        self.renderers.push(renderer);
        self
    }

    pub fn build(self, settings : RendererOptions, window : Window) -> RendererOrchestrator {
        let surface = Surface::new(&self.context, &window);
        let (physical_device, graphics_queue, presentation_queue, transfer_queue) = select(&self.context, &surface, &settings);
        let queue_families = { // Deduplicate the graphics and presentation queues.
            let mut queue_families_map = IntMap::<u32, QueueFamily>::default();
            queue_families_map.entry(graphics_queue.index()).or_insert(graphics_queue);
            queue_families_map.entry(presentation_queue.index()).or_insert(presentation_queue);
            queue_families_map.entry(transfer_queue.index()).or_insert(transfer_queue);

            queue_families_map.into_values().collect::<Vec<_>>()
        };

        let device = physical_device.create_logical_device(
            &self.context,
            queue_families.iter()
                .map(|queue : &QueueFamily| ((settings.get_queue_count)(queue), queue))
                .collect::<Vec<_>>(),
            |_index, _family| 1.0_f32,
            &settings.device_extensions,
            &surface,
        );

        // TODO: I get why I have to do this but the swapchain should take care not to consider the transfer queue selected here
        let swapchain_queue_families = queue_families.iter()
            .filter(|q| q.index() == graphics_queue.index() || q.index() == presentation_queue.index())
            .cloned()
            .collect::<Vec<_>>();

        let swapchain = Swapchain::new(&self.context, &device, &surface, &settings, swapchain_queue_families);
        
        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: self.context.handle().clone(),
            device: device.handle().clone(),
            physical_device: physical_device.handle().clone(),

            // TODO: All these may need tweaking and fixing
            debug_settings: AllocatorDebugSettings::default(),
            allocation_sizes : AllocationSizes::default(),
            buffer_device_address: false,
        }).unwrap();

        let pipeline_cache = Arc::new(PipelinePool::new(device.clone(), (settings.get_pipeline_cache_file)()));
        
        let context = Arc::new(RenderingContext {
            context : self.context.clone(),
            surface,
            device,
            graphics_queue,
            presentation_queue,
            transfer_queue,
            swapchain,
            allocator : ManuallyDrop::new(Arc::new(Mutex::new(allocator))),
            pipeline_cache,
            window,

            options : settings,
        });

        self.finalize(context)
    }

    fn finalize(self, context : Arc<RenderingContext>) -> RendererOrchestrator {
        let mut framebuffers = vec![];
        let mut created_renderers = vec![];
        let renderer_count = self.renderers.len();
        for renderer in &self.renderers {
            let mut renderer = renderer(&context);

            framebuffers.extend(renderer.create_framebuffers(&context.swapchain));
            created_renderers.push(renderer);
        }

        assert_eq!(renderer_count * context.swapchain.image_count(), framebuffers.len());

        // Create frame data
        let frames = {
            let mut frames = Vec::<FrameData>::with_capacity(context.swapchain.image_count());
            for i in 0..context.swapchain.image_count() {
                frames.push(FrameData::new(i, &context.device));
            }
            frames
        };

        RendererOrchestrator {
            context : context.clone(),

            renderers : created_renderers,
            framebuffers,

            // Frame-specific data
            frames,
            frame_index : 0,
            image_index : 0
        }
    }
}

pub struct RendererOrchestrator {
    pub context : Arc<RenderingContext>,

    renderers : Vec<Box<dyn Renderer>>,
    // This should be a bidimensional array but for the sake of memory layout, we use a single dimensional array.
    // The layout is effectively [renderer 1's framebuffers], [renderer 2's framebuffers], ...
    framebuffers : Vec<Framebuffer>,
    
    frames : Vec<FrameData>,
    image_index : usize,
    frame_index : usize,
}
impl RendererOrchestrator {
    pub fn draw_frame(&mut self) -> Result<(), RendererError> {
        let (image_acquired, _) = self.acquire_image()?;
        let frame = &self.frames[self.frame_index];

        frame.cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        for i in 0..self.renderers.len() {
            let renderer = &mut self.renderers[i];
            let framebuffer = &self.framebuffers[self.frames.len() * i + self.frame_index];

            frame.cmd.begin_label(renderer.marker_label(), renderer.marker_color());
            renderer.record_commands(framebuffer, frame);
            frame.cmd.end_label();
        }
        frame.cmd.end();

        let signal_semaphore = self.submit_frame(&[(image_acquired, vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)]);
        self.present_frame(signal_semaphore)?;

        Ok(())
    }

    pub fn handle_event(&mut self, event : &WindowEvent) {
        let mut repaint_instructions = Vec::<bool>::with_capacity(self.renderers.len());
        for i in 0..self.renderers.len() {
            let event_response = self.renderers[i].handle_event(event);
            repaint_instructions.push(event_response.repaint);
            if event_response.consumed {
                break;
            }
        }

        // TOOD: do somethign with the repaint instructions.
    }

    fn acquire_image(&mut self) -> Result<(vk::Semaphore, usize), RendererError> {
        self.context.device.wait_for_fence(self.frames[self.frame_index].in_flight);

        let acquired_semaphore = self.frames[self.frame_index].image_available;

        let image_index = match self.context.swapchain.acquire_image(acquired_semaphore, vk::Fence::null(), u64::MAX) {
            Ok((image_index, _)) => image_index,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                return Err(RendererError::InvalidSwapchain);
            },
            Err(vk::Result::SUBOPTIMAL_KHR) => {
                return Err(RendererError::InvalidSwapchain);
            },
            Err(error) => panic!("Error while acquiring next image: {:?}", error)
        };

        assert!((image_index as usize) < self.frames.len());
        self.image_index = image_index as _;

        // Set the image index returned by acquisition as the current frame.
        self.context.device.reset_fences(slice::from_ref(&self.frames[self.frame_index].in_flight));

        Ok((acquired_semaphore, self.frame_index))
    }

    fn submit_frame(&mut self, wait_info : &[(vk::Semaphore, vk::PipelineStageFlags)]) -> vk::Semaphore {
        let signal_semaphore = self.frames[self.frame_index].render_finished;

        let graphics_queue = self.context.device.get_queues(QueueAffinity::Graphics)[0];
        self.context.device.submit(graphics_queue,
            &[
                &self.frames[self.frame_index].cmd
            ],
            wait_info,
            &[signal_semaphore],
            self.frames[self.frame_index].in_flight
        );
    
        signal_semaphore
    }

    fn present_frame(&mut self, wait_semaphore: vk::Semaphore) -> Result<(), RendererError> {
        let wait_semaphores = [wait_semaphore];
        let swapchains = [self.context.swapchain.handle()];
        let image_indices = [self.image_index as u32];

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            let presentation_queue = self.context.device.get_queues(QueueAffinity::Graphics)[0]; // TODO: Use the present queue here, not the graphics queue
            let result = self.context.swapchain.loader
                .queue_present(presentation_queue.handle(), &present_info);

            self.frame_index = (self.frame_index + 1) % self.frames.len();
            self.frames[self.frame_index].semaphore_pool.reset();

            match result {
                Ok(_) => Ok(()),
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Err(RendererError::InvalidSwapchain),
                Err(vk::Result::SUBOPTIMAL_KHR) => Err(RendererError::InvalidSwapchain),
                Err(error) => panic!("Error while presenting frame: {:?}", error)
            }
        }
    }
}

/// Selects a [`PhysicalDevice`] and its associated graphics and presentation [`queue families`](QueueFamily).
///
/// Device selection is done according to its classification, with the following order:
///
/// 1. [`vk::PhysicalDeviceType::DISCRETE_GPU`]
/// 2. [`vk::PhysicalDeviceType::INTEGRATED_GPU`]
/// 3. [`vk::PhysicalDeviceType::VIRTUAL_GPU`]
/// 4. [`vk::PhysicalDeviceType::CPU`]
/// 5. [`vk::PhysicalDeviceType::OTHER`]
///
/// If possible, the graphics and presentation queue families will be the same to reduce internal synchronization.
fn select(context : &Arc<Context>, surface : &Arc<Surface>, settings : &RendererOptions) -> (PhysicalDevice, QueueFamily, QueueFamily, QueueFamily) {
    context.get_physical_devices(|left, right| {
        // DISCRETE_GPU > INTEGRATED_GPU > VIRTUAL_GPU > CPU > OTHER
        match (right.properties().device_type, left.properties().device_type) {
            // Base equality case
            (a, b) if a == b => Ordering::Equal,

            // DISCRETE_GPU > ALL
            (vk::PhysicalDeviceType::DISCRETE_GPU, _) => Ordering::Greater,

            // DISCRETE > INTEGRATED > ALL
            (vk::PhysicalDeviceType::INTEGRATED_GPU, vk::PhysicalDeviceType::DISCRETE_GPU) => Ordering::Less,
            (vk::PhysicalDeviceType::INTEGRATED_GPU, _) => Ordering::Greater,

            // DISCRETE, INTEGRATED > VIRTUAL > ALL
            (vk::PhysicalDeviceType::VIRTUAL_GPU, vk::PhysicalDeviceType::DISCRETE_GPU) => Ordering::Less,
            (vk::PhysicalDeviceType::VIRTUAL_GPU, vk::PhysicalDeviceType::INTEGRATED_GPU) => Ordering::Less,
            (vk::PhysicalDeviceType::VIRTUAL_GPU, _) => Ordering::Greater,

            // DISCRETE, INTEGRATED, VIRTUAL > CPU > ALL
            (vk::PhysicalDeviceType::CPU, vk::PhysicalDeviceType::DISCRETE_GPU) => Ordering::Less,
            (vk::PhysicalDeviceType::CPU, vk::PhysicalDeviceType::INTEGRATED_GPU) => Ordering::Less,
            (vk::PhysicalDeviceType::CPU, vk::PhysicalDeviceType::VIRTUAL_GPU) => Ordering::Less,
            (vk::PhysicalDeviceType::CPU, _) => Ordering::Greater,

            // ALL > OTHER
            (vk::PhysicalDeviceType::OTHER, _) => Ordering::Less,

            // Default case for branch solver
            (_, _) => unsafe { hint::unreachable_unchecked() },
        }
    })
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
    }).find_map(|device| {
        // At this point, the current device is eligible and we just need to check for a present queue and a graphics queue.
        // To do that, we will grab the queue's families.

        let mut graphics_queue = None;
        let mut present_queue = None;
        let mut transfer_queue = None;

        for family in &device.queue_families[..] {
            if family.is_graphics() {
                graphics_queue = Some(family.clone());

                // If this family can present as well just use it as a graphics+present queue
                if family.can_present(&surface, device.handle()) {
                    present_queue = Some(family.clone());
                }
            }

            // Default to the first available present queue
            if family.can_present(&surface, device.handle()) && present_queue.is_none() {
                present_queue = Some(family.clone());
            }

            // If this family can transfer and no transfer queue is found,
            // If this family can transfer and is only a transfer queue
            if family.is_transfer() && ((!family.is_graphics() && !family.is_compute()) || transfer_queue.is_none()) {
                transfer_queue = Some(family.clone());
            }
        }

        match (graphics_queue, present_queue, transfer_queue) {
            (Some(g), Some(p), Some(t)) => Some((device, g, p, t)),
            _ => None
        }
    }).expect("Failed to select a physical device and an associated queue family")
}
