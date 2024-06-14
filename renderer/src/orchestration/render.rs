use std::{ffi::CString, mem::ManuallyDrop, slice, sync::Arc};

use ash::vk;
use egui::ahash::HashMapExt;
use nohash_hasher::IntMap;
use puffin::profile_scope;

use crate::{application::RendererError, traits::handle::Handle, vk::{context::Context, frame_data::FrameData, logical_device::LogicalDevice, queue::{QueueAffinity, QueueFamily}, renderer::RendererOptions, swapchain::Swapchain}, window::Window};

use super::rendering::{Renderable, RenderingContext, RenderingContextImpl};

pub struct RendererCallbacks {
    context : Arc<Context>,
}
impl RendererCallbacks {
    pub fn new(context : Arc<Context>) -> Self {
        Self {
            context,
        }
    }

    pub fn build<'a>(&'a self,
        options : RendererOptions,
        window : Window,
        device_extensions : Vec<CString>,
    ) -> Renderer {
        let (device, graphics_queue, presentation_queue, transfer_queue) = self.create_device(&window, &options, device_extensions);

        let context = Arc::new(RenderingContextImpl {
            context : self.context.clone(),
            window,

            device,
            graphics_queue,
            presentation_queue,
            transfer_queue,

            options,
        });

        let swapchain = Swapchain::new(&context, &options, vec![graphics_queue, presentation_queue]);

        let mut frames = Vec::<FrameData>::with_capacity(swapchain.image_count());
        for i in 0..swapchain.image_count() {
            frames.push(FrameData::new(i, &context));
        }

        Renderer {
            context,
            swapchain : ManuallyDrop::new(swapchain),

            frames,
            frame_index : 0,
            image_index : 0,
        }
    }
    
    fn create_device(&self, window : &Window, settings : &RendererOptions, device_extensions : Vec<CString>)
        -> (LogicalDevice, QueueFamily, QueueFamily, QueueFamily)
    {
        let (physical_device, graphics_queue, presentation_queue, transfer_queue) = self.context.select_physical_device(&window, &device_extensions);

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
                .map(|queue| ((settings.get_queue_count)(queue), queue))
                .collect::<Vec<_>>(),
            |_index, _family| 1.0_f32,
            &device_extensions,
            (settings.get_pipeline_cache_file)(),
            &window,
        );

        (device, graphics_queue, presentation_queue, transfer_queue)
    }
}

pub struct Renderer {
    pub context: RenderingContext,
    pub swapchain : ManuallyDrop<Swapchain>,
    frames : Vec<FrameData>,
    frame_index : usize,
    image_index : usize,
}

pub struct RendererUpdater<'a> {
    renderer : &'a mut Renderer,
    objects : Vec<&'a mut dyn Renderable>,
}
impl RendererUpdater<'_> {
    pub fn recreate_swapchain(&mut self) {
        self.renderer.context.device.wait_idle();
        self.renderer.frames.clear();

        unsafe {
            ManuallyDrop::drop(&mut self.renderer.swapchain);
        }

        self.renderer.swapchain = ManuallyDrop::new(Swapchain::new(&self.renderer.context, &self.renderer.context.options, vec![
            self.renderer.context.graphics_queue,
            self.renderer.context.presentation_queue
        ]));

        for renderer in &mut self.objects {
            renderer.create_framebuffers(&self.renderer.swapchain);
        }

        self.renderer.frames = Vec::<FrameData>::with_capacity(self.renderer.swapchain.image_count());
        for i in 0..self.renderer.swapchain.image_count() {
            self.renderer.frames.push(FrameData::new(i, &self.renderer.context));
        }
    }

    pub fn draw(&mut self) -> Result<(), RendererError> {
        profile_scope!("Frame draw calls");

        let (image_acquired, _) = self.renderer.acquire_image()?;
        let frame = &self.renderer.frames[self.renderer.frame_index];

        frame.cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        for renderer in &mut self.objects {
            let marker = renderer.marker_data();

            profile_scope!("Renderer draw calls", marker.0);

            frame.cmd.begin_label(marker.0, marker.1);
            renderer.record_commands(&self.renderer.swapchain, frame);
            frame.cmd.end_label();
        }
        frame.cmd.end();

        self.renderer.end_frame(image_acquired)
    }
}

impl Renderer {
    pub fn builder(context : Arc<Context>) -> RendererCallbacks {
        RendererCallbacks::new(context)
    }

    /// Returns an updatability handling type that provides methods to properly manipulate rendering.
    /// 
    /// # Arguments
    /// 
    /// * `renderers` - An array of renderables to associate with this renderer.
    pub fn updater<'a>(&'a mut self, renderers : Vec<&'a mut dyn Renderable>) -> RendererUpdater<'a> {
        RendererUpdater::<'a> {
            objects : renderers,
            renderer : self,
        }
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

    fn end_frame(&mut self, image_acquired : vk::Semaphore) -> Result<(), RendererError> {
        let signal_semaphore = self.submit_frame(&[(image_acquired, vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)]);
        self.present_frame(signal_semaphore)
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
}

pub trait RendererAPI {
    fn is_minimized(&self) -> bool;

    fn recreate_swapchain(&mut self);

    fn wait_idle(&self);
}