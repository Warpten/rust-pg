use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::ffi::CStr;
use std::{hint, slice};
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};
use ash::vk;
use gpu_allocator::{AllocationSizes, AllocatorDebugSettings};
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use nohash_hasher::IntMap;
use crate::application::ApplicationRenderError;
use crate::orchestration::traits::RenderableFactory;
use crate::traits::handle::Handle;
use crate::vk::command_buffer::CommandBuffer;
use crate::vk::context::Context;
use crate::vk::frame_data::FrameData;
use crate::vk::logical_device::LogicalDevice;
use crate::vk::physical_device::PhysicalDevice;
use crate::vk::pipeline::pool::PipelinePool;
use crate::vk::queue::{QueueAffinity, QueueFamily};
use crate::vk::render_pass::RenderPass;
use crate::vk::renderer::RendererOptions;
use crate::vk::surface::Surface;
use crate::vk::swapchain::Swapchain;
use crate::window::Window;

use super::traits::{Renderable, RenderableFactoryProvider};

/// Conceptually, a [`Renderer`] is a gigantic wrapper around a [`vk::RenderPass`]. Each [`Renderable`] is effectively treated
/// as a subpass.
pub struct RendererBuilder {
    context : Arc<Context>,
    renderables : Vec<RenderableFactoryProvider>,
}

impl RendererBuilder {
    pub fn default(context : &Arc<Context>) -> Self {
        Self {
            context : context.clone(),
            renderables : vec![]
        }
    }

    value_builder! { context, Arc<Context> }

    /// Adds a new renderable. A renderable is able to influence the render pass's creation as well as inject draw calls in a frame.
    /// Each renderable behaves as if it was a subpass.
    pub fn add_renderable(&mut self, renderable : RenderableFactoryProvider) {
        self.renderables.push(renderable);
    }

    pub fn build(self, settings : RendererOptions, window : Window) -> Renderer {
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

        let renderables : Vec<Box<dyn RenderableFactory>> = self.renderables.into_iter()
            .map(|factory| factory(&device, &swapchain, &pipeline_cache))
            .collect::<Vec<_>>();

        let render_pass = {
            // Prepare a render pass - the swapchain just declares attachments but does not express dependencies or subpasses.
            let mut create_info = swapchain.create_render_pass();
            for renderable in &renderables {
                create_info = renderable.express_dependencies(create_info);
            }
            create_info.build(&device)
        };

        let mut framebuffers = swapchain.create_framebuffers(&render_pass);

        let frames = {
            let mut frames = Vec::<FrameData>::with_capacity(swapchain.image_count());
            for (i, framebuffer) in framebuffers.drain(..).enumerate() {
                frames.push(FrameData::new(i, &device, framebuffer));
            }
            frames
        };

        let clear_values = swapchain.get_clear_values(&settings);

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
            render_pass,
            window,

            options : settings,
        });

        let renderables = renderables.into_iter()
            .map(|renderable| renderable.build(&context))
            .collect::<Vec<_>>();

        Renderer {
            context,
            renderables,
            clear_values,
            
            frames,
            frame_index : 0,
            image_index : 0,
        }
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
    pub render_pass : RenderPass,
    pub window : Window,

    pub options : RendererOptions,
}

pub struct Renderer {
    pub context : Arc<RenderingContext>,

    renderables : Vec<Box<dyn Renderable>>,
    clear_values : Vec<vk::ClearValue>,

    frames : Vec<FrameData>,
    image_index : usize,
    frame_index : usize,
}

impl Renderer {
    /// Takes care of drawing an entire frame.
    pub fn draw_frame(&mut self) -> Result<(), ApplicationRenderError> {
        let (image_acquired, _) = self.acquire_next_image()?;

        let frame = &self.frames[self.frame_index]; // Current frame data.

        let cmd = CommandBuffer::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .pool(frame.graphics_command_pool.as_ref().unwrap())
            .build_one(&self.context.device);

        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        cmd.begin_render_pass(&self.context.render_pass, &frame.framebuffer, vk::Rect2D {
            offset : vk::Offset2D { x: 0, y : 0 },
            extent : self.context.swapchain.extent
        }, &self.clear_values, self.renderables[0].contents_type());

        for i in 0..self.renderables.len() {
            if i > 0 {
                cmd.next_subpass(self.renderables[i].contents_type());
            }

            self.renderables[i].draw_frame(&cmd, self.frame_index);
        }

        cmd.end_render_pass();
        cmd.end();
        self.submit_and_present(&cmd, image_acquired)?;

        Ok(())
    }

    // TODO: rework this to record/replay by reusing command buffers across frames
    /// Begins a new frame.
    /// 
    /// # Arguments
    /// 
    /// * `render_pass` - The render pass that is the subject of this call.
    pub fn begin_frame(&mut self, render_pass : &RenderPass) -> Result<(vk::Semaphore, CommandBuffer), ApplicationRenderError> {
        let (image_acquired, _) = self.acquire_next_image()?;

        let frame = &self.frames[self.frame_index];

        let cmd = CommandBuffer::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .pool(frame.graphics_command_pool.as_ref().unwrap())
            .build_one(&self.context.device);
        
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        cmd.begin_render_pass(render_pass, &frame.framebuffer, vk::Rect2D {
            offset : vk::Offset2D { x: 0, y : 0 },
            extent : self.context.swapchain.extent
        }, &self.clear_values, vk::SubpassContents::INLINE);

        Ok((image_acquired, cmd))
    }

    /// Ends the current frame.
    pub fn end_frame(&mut self, image_acquired : vk::Semaphore, cmd : &CommandBuffer) -> Result<(), ApplicationRenderError> {
        cmd.end_render_pass();
        cmd.end();
        self.submit_and_present(cmd, image_acquired)
    }

    /// Acquires the next available image from the swapchain. Returns a semaphore that will be signaled when the image is acquired as well
    /// as the index of said image in the swapchain. This index is different from the frame index.
    /// 
    /// This function returns a semaphore that will be signalled when an image is available.
    pub fn acquire_next_image(&mut self) -> Result<(vk::Semaphore, usize), ApplicationRenderError> {
        self.context.device.wait_for_fence(self.frames[self.frame_index].in_flight);

        let acquired_semaphore = self.frames[self.frame_index].image_available;

        let image_index = match self.context.swapchain.acquire_image(acquired_semaphore, vk::Fence::null(), u64::MAX) {
            Ok((image_index, _)) => image_index,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                return Err(ApplicationRenderError::InvalidSwapchain);
            },
            Err(vk::Result::SUBOPTIMAL_KHR) => {
                return Err(ApplicationRenderError::InvalidSwapchain);
            },
            Err(error) => panic!("Error while acquiring next image: {:?}", error)
        };

        assert!((image_index as usize) < self.frames.len());
        self.image_index = image_index as _;

        // Set the image index returned by acquisition as the current frame.
        self.context.device.reset_fences(slice::from_ref(&self.frames[self.frame_index].in_flight));

        Ok((acquired_semaphore, self.frame_index))
    }

    /// Submits a frame.
    /// 
    /// # Arguments
    /// 
    /// * `command_buffers` - A slice of command buffers to execute in batch.
    /// * `wait_semaphores` - Semaphores upon which to wait before executing the command buffers.
    /// * `flags` - An array of pipeline stages at which each corresponding semaphore wait will occur.
    /// 
    /// # Returns
    /// 
    /// A [`vk::Semaphore`] that will be signalled when all command buffers have completed execution.
    pub fn submit_frame(&mut self, command_buffers : &[&CommandBuffer], wait_info : &[(vk::Semaphore, vk::PipelineStageFlags)]) -> vk::Semaphore {
        let signal_semaphore = self.frames[self.frame_index].render_finished;

        let graphics_queue = self.context.device.get_queues(QueueAffinity::Graphics)[0];
        self.context.device.submit(graphics_queue, command_buffers, wait_info, &[signal_semaphore], self.frames[self.frame_index].in_flight);
    
        signal_semaphore
    }

    /// Presents the current image to the swapchain.
    /// 
    /// # Arguments
    /// 
    /// * `wait_semaphore` - The semaphore to wait on in order to present the image that was acquired.
    pub fn present_frame(&mut self, wait_semaphore: vk::Semaphore) -> Result<(), ApplicationRenderError> {
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
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Err(ApplicationRenderError::InvalidSwapchain),
                Err(vk::Result::SUBOPTIMAL_KHR) => Err(ApplicationRenderError::InvalidSwapchain),
                Err(error) => panic!("Error while presenting frame: {:?}", error)
            }
        }
    }
    
    /// Submits the provided command buffer to the graphics queue and presents the image to the swapchain.
    /// 
    /// # Arguments
    /// 
    /// * `cmd` - The command buffer to submit.
    /// * `wait_semaphore` - The semaphore to wait on. This semaphore is signalled once an image has been acquired from the swapchain.
    pub fn submit_and_present(&mut self, cmd : &CommandBuffer, wait_semaphore : vk::Semaphore) -> Result<(), ApplicationRenderError> {
        let signal_semaphore = self.submit_frame(&[cmd], &[(wait_semaphore, vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)]);

        self.present_frame(signal_semaphore)
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
