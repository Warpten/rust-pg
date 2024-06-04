use std::ffi::CString;
use std::mem::ManuallyDrop;
use std::slice;
use std::sync::{Arc, Mutex};

use ash::vk::{self};
use egui::ahash::HashMapExt;
use egui_winit::winit::event::WindowEvent;
use egui_winit::EventResponse;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use gpu_allocator::{AllocationSizes, AllocatorDebugSettings};
use nohash_hasher::IntMap;
use puffin::profile_scope;

use crate::application::RendererError;
use crate::traits::handle::Handle;
use crate::vk::context::Context;
use crate::vk::frame_data::FrameData;
use crate::vk::framebuffer::Framebuffer;
use crate::vk::logical_device::LogicalDevice;
use crate::vk::pipeline::pool::PipelinePool;
use crate::vk::queue::{QueueAffinity, QueueFamily};
use crate::vk::renderer::RendererOptions;
use crate::vk::swapchain::Swapchain;
use crate::window::Window;

/// A renderer is effectively a type that declares the need to work with its own render pass.
pub trait Renderer {
    /// Returns a recorded command buffer that contains all the commands needed to render the contents of this renderer.
    fn record_commands(&mut self, swapchain : &Swapchain, framebuffer : &Framebuffer, frame_data : &FrameData);
    fn create_framebuffers(&self, swapchain : &Swapchain) -> Vec<Framebuffer>;

    fn marker_label(&self) -> String;
    fn marker_color(&self) -> [f32; 4];

    fn handle_event(&mut self, event : &WindowEvent) -> EventResponse {
        EventResponse { repaint : false, consumed : false }
    }
}

pub struct RenderingContext {
    context : Arc<Context>,
    pub device : Arc<LogicalDevice>,
    pub graphics_queue : QueueFamily,
    pub presentation_queue : QueueFamily,
    pub transfer_queue : QueueFamily,
    allocator : ManuallyDrop<Arc<Mutex<Allocator>>>,
    pub pipeline_cache : Arc<PipelinePool>,
    pub window : Arc<Window>,

    pub options : RendererOptions,
}
pub type SynchronizedRenderingContext = Arc<RenderingContext>;

pub type RendererFn = fn(context : &SynchronizedRenderingContext, swapchain : &Swapchain) -> Box<dyn Renderer>;

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

    pub fn build(self,
        settings : RendererOptions,
        window : &Arc<Window>,
        device_extensions : Vec<CString>,
    ) -> RendererOrchestrator {
        let (device, graphics_queue, presentation_queue, transfer_queue) = self.create_device(&window, &settings, device_extensions);
        let swapchain = self.context.create_swapchain(&device, &window, &settings, vec![graphics_queue, presentation_queue]);
        
        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: self.context.handle().clone(),
            device: device.handle().clone(),
            physical_device: device.physical_device.handle().clone(),

            // TODO: All these may need tweaking and fixing
            debug_settings: AllocatorDebugSettings::default(),
            allocation_sizes : AllocationSizes::default(),
            buffer_device_address: false,
        }).unwrap();

        let pipeline_cache = Arc::new(PipelinePool::new(device.clone(), (settings.get_pipeline_cache_file)()));
        
        let context = Arc::new(RenderingContext {
            context : self.context.clone(),
            device,
            graphics_queue,
            presentation_queue,
            transfer_queue,
            allocator : ManuallyDrop::new(Arc::new(Mutex::new(allocator))),
            pipeline_cache,
            window : window.clone(),

            options : settings,
        });

        let (renderers, framebuffers, frames) = self.create_frame_data(&swapchain, &context);
        
        RendererOrchestrator {
            context : context.clone(),
            swapchain : ManuallyDrop::new(swapchain),

            renderers,
            framebuffers,
            frames,
            frame_index : 0,
            image_index : 0
        }
    }

    fn create_device(&self, window : &Arc<Window>, settings : &RendererOptions, device_extensions : Vec<CString>)
        -> (Arc<LogicalDevice>, QueueFamily, QueueFamily, QueueFamily)
    {
        let (physical_device, graphics_queue, presentation_queue, transfer_queue) = self.context.select_physical_device(&window, &settings, &device_extensions);

        let queue_families = { // Deduplicate the graphics and presentation queues.
            let mut queue_families_map = IntMap::<u32, QueueFamily>::with_capacity(3);
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
            &device_extensions,
            &window,
        );

        (device, graphics_queue, presentation_queue, transfer_queue)
    }

    fn create_frame_data(&self, swapchain : &Swapchain, context : &SynchronizedRenderingContext) -> (Vec<Box<dyn Renderer>>, Vec<Framebuffer>, Vec<FrameData>) {
        let mut framebuffers = vec![];
        let mut created_renderers = vec![];
        let renderer_count = self.renderers.len();
        for renderer in &self.renderers {
            let mut renderer = renderer(context, swapchain);

            framebuffers.extend(renderer.create_framebuffers(&swapchain));
            created_renderers.push(renderer);
        }

        assert_eq!(renderer_count * swapchain.image_count(), framebuffers.len());

        // Create frame data
        let frames = {
            let mut frames = Vec::<FrameData>::with_capacity(swapchain.image_count());
            for i in 0..swapchain.image_count() {
                frames.push(FrameData::new(i, &context.device));
            }
            frames
        };

        (created_renderers, framebuffers, frames)
    }
}

pub struct RendererOrchestrator {
    pub context : Arc<RenderingContext>,
    pub swapchain : ManuallyDrop<Swapchain>,

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
        profile_scope!("Application rendering");

        let (image_acquired, _) = self.acquire_image()?;
        let frame = &self.frames[self.frame_index];

        frame.cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        for i in 0..self.renderers.len() {
            profile_scope!("Render pass", i.to_string());

            let renderer = &mut self.renderers[i];
            let framebuffer = &self.framebuffers[self.frames.len() * i + self.frame_index];

            frame.cmd.begin_label(renderer.marker_label(), renderer.marker_color());
            renderer.record_commands(&self.swapchain, framebuffer, frame);
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
        profile_scope!("Frame acquisition");

        self.context.device.wait_for_fence(self.frames[self.frame_index].in_flight);

        let acquired_semaphore = self.frames[self.frame_index].image_available;

        let image_index = match self.swapchain.acquire_image(acquired_semaphore, vk::Fence::null(), u64::MAX) {
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
        profile_scope!("Frame submission");

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
        profile_scope!("Frame presentation");

        let wait_semaphores = [wait_semaphore];
        let swapchains = [self.swapchain.handle()];
        let image_indices = [self.image_index as u32];

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            let presentation_queue = self.context.device.get_queues(QueueAffinity::Graphics)[0]; // TODO: Use the present queue here, not the graphics queue
            let result = self.swapchain.loader
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

    pub fn recreate_swapchain(&mut self) {
        self.context.device.wait_idle();

        self.framebuffers.clear();
        self.frames.clear();

        unsafe {
            ManuallyDrop::drop(&mut self.swapchain);
        }

        self.swapchain = ManuallyDrop::new(self.context.context.create_swapchain(&self.context.device, &self.context.window, &self.context.options, vec![
            self.context.graphics_queue,
            self.context.presentation_queue
        ]));

        for renderer in &mut self.renderers {
            self.framebuffers.extend(renderer.create_framebuffers(&self.swapchain));
        }

        self.frames = Vec::<FrameData>::with_capacity(self.swapchain.image_count());
        for i in 0..self.swapchain.image_count() {
            self.frames.push(FrameData::new(i, &self.context.device));
        }

        // I think that's it? Everything should drop.
    }
}
