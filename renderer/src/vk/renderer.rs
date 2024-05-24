use std::{cmp::Ordering, collections::HashSet, ffi::{CStr, CString}, hint, mem::ManuallyDrop, path::PathBuf, sync::{Arc, Mutex}};

use ash::vk::{self, ClearValue};
use gpu_allocator::{AllocationSizes, AllocatorDebugSettings, vulkan::{Allocator, AllocatorCreateDesc}};
use nohash_hasher::IntMap;

use crate::{application::ApplicationRenderError, graph::Graph, traits::handle::{BorrowHandle, Handle}, vk::{Context, LogicalDevice, PhysicalDevice, PipelinePool, QueueFamily, Surface, Swapchain, SwapchainOptions}, window::Window};
use crate::vk::{frame_data::FrameData, Framebuffer, QueueAffinity, RenderPass};

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
    pub(in crate) depth : bool,
    pub(in crate) stencil : bool,
    pub(in crate) separate_depth_stencil : bool, // NYI
    pub(in crate) clear_color : [f32; 4],
    pub(in crate) multisampling : vk::SampleCountFlags,
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

    #[inline] pub fn depth(mut self, depth : bool) -> Self {
        self.depth = depth;
        self
    }

    #[inline] pub fn stencil(mut self, stencil : bool) -> Self {
        self.stencil = stencil;
        self
    }

    #[inline] pub fn clear_color(mut self, clear_color : [f32; 4]) -> Self {
        self.clear_color = clear_color;
        self
    }

    #[inline] pub fn multisampling(mut self, samples : vk::SampleCountFlags) -> Self {
        self.multisampling = samples;
        self
    }
}

impl Default for RendererOptions {
    fn default() -> Self {
        Self {
            line_width: DynamicState::Fixed(1.0f32),
            device_extensions: vec![
                ash::khr::swapchain::NAME.to_owned(),
            ],
            instance_extensions: vec![],
            resolution : [1280, 720],
            get_queue_count : |&_| 1,
            get_pipeline_cache_file : || "pipelines.dat".into(),
            depth : true,
            stencil : true,
            separate_depth_stencil : false,
            clear_color : [0.0f32, 0.0f32, 0.0f32, 0.0f32],
            multisampling : vk::SampleCountFlags::TYPE_1,
        }
    }
}

impl SwapchainOptions for RendererOptions {
    fn select_surface_format(&self, format : &vk::SurfaceFormatKHR) -> bool {
        format.format == vk::Format::B8G8R8A8_SRGB && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
    }

    fn width(&self) -> u32 { self.resolution[0] }
    fn height(&self) -> u32 { self.resolution[1] }

    fn present_mode(&self) -> vk::PresentModeKHR { vk::PresentModeKHR::MAILBOX }

    fn depth(&self) -> bool { self.depth }
    fn stencil(&self) -> bool { self.stencil }
    fn multisampling(&self) -> vk::SampleCountFlags { self.multisampling }
}

pub struct Renderer {
    // Internal Vulkan types
    context : Arc<Context>,
    logical_device : Arc<LogicalDevice>,
    pipeline_cache : Arc<PipelinePool>,
    surface : Arc<Surface>,
    swapchain : Arc<Swapchain>,
    render_pass : RenderPass,
    allocator : ManuallyDrop<Arc<Mutex<Allocator>>>,

    // Actual application stuff
    framebuffers : Vec<Framebuffer>,
    clear_values : [ClearValue; 2],
    frames : Vec<FrameData>,
    active_frame_index : usize,

    // One or many rendering graphs
    // The application driving the renderer is in charge of adding as many graphs as needed. They will be
    // baked, invalidated, scheduled and executed in order.
    graphs : Vec<Graph>,
}

impl Renderer {
    #[inline] pub fn context(&self) -> &Context { &self.context }
    #[inline] pub fn logical_device(&self) -> &Arc<LogicalDevice> { &self.logical_device }
    #[inline] pub fn pipeline_cache(&self) -> &Arc<PipelinePool> { &self.pipeline_cache }
    #[inline] pub fn surface(&self) -> &Arc<Surface> { &self.surface }
    #[inline] pub fn swapchain(&self) -> &Arc<Swapchain> { &self.swapchain }
    #[inline] pub fn allocator(&self) -> &Arc<Mutex<Allocator>> { &self.allocator }

    pub fn new(settings : &RendererOptions, context: &Arc<Context>, window : &Window) -> Self {
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
            queue_families,
        );
        let render_pass = swapchain.create_render_pass();
        let framebuffers = swapchain.create_framebuffers(&render_pass);

        let frames = {
            let mut frames = Vec::<FrameData>::new();

            for i in 0..swapchain.image_count() {
                frames.push(FrameData::new(i, &logical_device));
            }

            frames
        };

        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: context.handle().clone(),
            device: logical_device.handle().clone(),
            physical_device: physical_device.handle().clone(),

            // TODO: All these may need tweaking and fixing
            debug_settings: AllocatorDebugSettings::default(),
            allocation_sizes : AllocationSizes::default(),
            buffer_device_address: false,
        }).unwrap();

        Self {
            context : context.clone(),
            pipeline_cache : Arc::new(PipelinePool::new(logical_device.clone(), (settings.get_pipeline_cache_file)())),
            logical_device,
            surface,
            swapchain,
            render_pass,
            framebuffers,
            allocator : ManuallyDrop::new(Arc::new(Mutex::new(allocator))),
            clear_values : [
                ClearValue {
                    color : vk::ClearColorValue {
                        float32: settings.clear_color,
                    },
                },
                ClearValue {
                    depth_stencil : vk::ClearDepthStencilValue {
                        depth : 1.0f32,
                        stencil : 0,
                    }
                }
            ],

            graphs : vec![],
            frames,
            active_frame_index : 0,
        }
    }

    pub fn acquire_next_image(&mut self) -> Result<(vk::Semaphore, usize), ApplicationRenderError> {
        let acquired_semaphore = self.frames[self.active_frame_index].semaphore_pool.request();

        let image_index = match self.swapchain().acquire_image(acquired_semaphore, vk::Fence::null(), u64::MAX) {
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

        // Set the image index returned by acquisition as the current frame.
        self.active_frame_index = image_index as _;
        self.frames[self.active_frame_index].semaphore_pool.reset();
        self.wait_and_reset(self.frames[self.active_frame_index].in_flight);

        Ok((acquired_semaphore, self.active_frame_index))
    }

    pub fn wait_and_reset(&self, fence : vk::Fence) {
        self.wait_for_fence(fence);
        self.reset_fence(fence);
    }

    pub fn wait_for_fence(&self, fence : vk::Fence) {
        unsafe {
            self.logical_device().handle().wait_for_fences(&[fence], true, u64::MAX)
                .expect("Waiting for the fence failed");
        }
    }

    pub fn reset_fence(&self, fence : vk::Fence) {
        unsafe {
            self.logical_device().handle().reset_fences(&[fence])
                .expect("Resetting the fence failed");
        }
    }

    pub fn submit_and_present(&mut self, command_buffer : vk::CommandBuffer, wait_semaphore : vk::Semaphore) -> Result<(), ApplicationRenderError> {
        let signal_semaphore = self.submit_frame(&[command_buffer],
            &[wait_semaphore],
            &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT]
        );

        self.present_frame(signal_semaphore)
    }

    pub fn begin_render_pass(&self, command_buffer : vk::CommandBuffer, extent : vk::Extent2D) {
        unsafe {
            let render_pass_begin_info = vk::RenderPassBeginInfo::default()
                .render_area(vk::Rect2D {
                    offset : vk::Offset2D { x: 0, y : 0 },
                    extent
                })
                .framebuffer(self.framebuffers[self.active_frame_index].handle())
                .render_pass(self.render_pass.handle())
                .clear_values(&self.clear_values);

            self.logical_device.handle().cmd_begin_render_pass(command_buffer, &render_pass_begin_info, vk::SubpassContents::INLINE);
        }
    }

    pub fn end_render_pass(&self, command_buffer : vk::CommandBuffer) {
        unsafe {
            self.logical_device.handle().cmd_end_render_pass(command_buffer);
        }
    }

    pub fn run_frame(&mut self, handler : fn(&ash::Device, vk::CommandBuffer)) -> Result<(), ApplicationRenderError> {
        let (image_acquired, _) = self.acquire_next_image().expect("Image acquisition failed");

        let cmd = self.begin_command_buffer(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        self.begin_render_pass(cmd, self.swapchain.extent);

        handler(self.logical_device.handle(), cmd);

        self.end_render_pass(cmd);
        self.end_command_buffer(cmd);
        self.submit_and_present(cmd, image_acquired)
    }

    pub fn begin_command_buffer(&mut self, flags : vk::CommandBufferUsageFlags) -> vk::CommandBuffer {
        let graphics_queue = self.logical_device.get_queues(QueueAffinity::Graphics, &self.surface)[0];

        self.frames[self.active_frame_index].reset_command_pool(graphics_queue.family());
        let cmd = self.frames[self.active_frame_index].get_command_buffer(graphics_queue.family(),
            vk::CommandBufferLevel::PRIMARY,
            1)[0];

        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(flags);

            self.logical_device.handle()
                .begin_command_buffer(cmd, &begin_info)
                .expect("Failed to begin frame commands");
        };

        cmd
    }

    pub fn end_command_buffer(&mut self, cmd : vk::CommandBuffer) {
        unsafe {
            self.logical_device.handle().end_command_buffer(cmd)
                .expect("Failed to end frame commands")
        };
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
    pub fn submit_frame(&mut self, command_buffers : &[vk::CommandBuffer], wait_semaphores : &[vk::Semaphore], flags : &[vk::PipelineStageFlags]) -> vk::Semaphore {
        let signal_semaphore = [
            self.frames[self.active_frame_index].semaphore_pool.request()
        ];

        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(wait_semaphores)
            .command_buffers(command_buffers)
            .wait_dst_stage_mask(flags)
            .signal_semaphores(&signal_semaphore);

        let graphics_queue = self.logical_device().get_queues(QueueAffinity::Graphics, self.surface())[0];
        self.logical_device().submit(&graphics_queue, &[submit_info], self.frames[self.active_frame_index].in_flight);
    
        signal_semaphore[0]
    }

    pub fn present_frame(&mut self, wait_semaphore: vk::Semaphore) -> Result<(), ApplicationRenderError> {
        let wait_semaphores = [wait_semaphore];
        let swapchains = [self.swapchain().handle()];
        let image_indices = [self.active_frame_index as u32];

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            let presentation_queue = self.logical_device.get_queues(QueueAffinity::Present, self.surface())[0];
            let result = self.swapchain.loader
                .queue_present(presentation_queue.handle(), &present_info);

            match result {
                Ok(_) => Ok(()),
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Err(ApplicationRenderError::InvalidSwapchain),
                Err(vk::Result::SUBOPTIMAL_KHR) => Err(ApplicationRenderError::InvalidSwapchain),
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
/// 
fn select(context : &Arc<Context>, surface : &Arc<Surface>, settings : &RendererOptions) -> (PhysicalDevice, QueueFamily, QueueFamily) {
    context.get_physical_devices(
        |left, right| {
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