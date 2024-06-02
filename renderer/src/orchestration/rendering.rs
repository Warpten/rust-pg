use std::mem::ManuallyDrop;
use std::slice;
use std::sync::{Arc, Mutex};

use ash::vk;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use gpu_allocator::{AllocationSizes, AllocatorDebugSettings};

use crate::application::RendererError;
use crate::traits::handle::Handle;
use crate::vk::command_buffer::{CommandBuffer, CommandBufferBuilder};
use crate::vk::context::Context;
use crate::vk::frame_data::FrameData;
use crate::vk::framebuffer::Framebuffer;
use crate::vk::logical_device::LogicalDevice;
use crate::vk::pipeline::pool::PipelinePool;
use crate::vk::queue::{QueueAffinity, QueueFamily};
use crate::vk::render_pass::RenderPass;
use crate::vk::renderer::RendererOptions;
use crate::vk::surface::Surface;
use crate::vk::swapchain::Swapchain;
use crate::window::Window;

/// A renderer is effectively a type that declares the need to work with its own render pass.
pub trait Renderer {
    /// Creates a render pass for this stage.
    /// 
    /// # Arguments
    /// 
    /// * `context` - The rendering context.
    /// * `is_presenting`` - Indicates if this stage is expected to present to whatever surface is used by the swapchain.
    fn create_render_pass(&self, context : &Arc<RenderingContext>, is_presenting : bool) -> RenderPass;

    /// Returns a recorded command buffer that contains all the commands needed to render the contents of this renderer.
    fn record_commands(&self, render_pass : &RenderPass, framebuffer : &Framebuffer, frame_data : &FrameData);

    fn marker_label(&self) -> String;
    fn marker_color(&self) -> [f32; 4];
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

pub struct Orchestrator {
    context : Arc<Context>,
    renderers : Vec<Box<dyn Renderer>>,
}
impl Orchestrator {
    /// Creates a new orchestrator. This object is in charge of preparing Vulkan structures for rendering
    /// as well as the way command buffers will be recorded and executed.
    pub fn new(context : Arc<Context>) -> Self {
        Self {
            context,
            renderers : vec![],
        }
    }

    /// Adds a renderable to this orchestrator. See the documentation on [`Renderer`] for more informations.
    pub fn add_renderer(mut self, renderer : Box<dyn Renderer>) -> Self {
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

        let frames = {
            let mut frames = Vec::<FrameData>::with_capacity(swapchain.image_count());
            for i in 0..swapchain.image_count() {
                frames.push(FrameData::new(i, &device));
            }
            frames
        };
        
        let context = Arc::new(RenderingContext {
            context : self.context,
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
        let mut render_passes = vec![];
        let mut framebuffers = vec![];
        let renderer_count = self.renderers.len();
        for (i, renderer) in self.renderers.iter().enumerate() {
            let render_pass = renderer.create_render_pass(&context, i + 1 == renderer_count);

            framebuffers.extend(context.swapchain.create_framebuffers(&render_pass));
            render_passes.push(render_pass);
        }

        assert_eq!(render_passes.len(), self.renderers.len());
        assert_eq!(render_passes.len() * context.swapchain.image_count(), framebuffers.len());

        // Create frame data

        RendererOrchestrator {
            context : context.clone(),

            renderers : self.renderers,
            render_passes,
            framebuffers,

            // Frame-specific data
            frames : vec![],
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
    render_passes : Vec<RenderPass>,
    
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
            let renderer = &self.renderers[i];
            let render_pass = &self.render_passes[i];
            let framebuffer = &self.framebuffers[self.frames.len() * i + self.frame_index];

            frame.cmd.begin_label(renderer.marker_label(), renderer.marker_color());
            renderer.record_commands(render_pass, framebuffer, frame);
            frame.cmd.end_label();
        }
        frame.cmd.end();

        let signal_semaphore = self.submit_frame(&[(image_acquired, vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)]);
        self.present_frame(signal_semaphore)?;

        Ok(())
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