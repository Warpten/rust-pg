use ash::vk;
use bytemuck::bytes_of;
use egui_winit::winit;
use std::fmt::Debug;
use std::mem::size_of;
use std::{
    collections::HashMap,
    fmt::Formatter,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
};

use crate::traits::handle::Handle;
use crate::vk::buffer::{Buffer, StaticBufferBuilder, StaticInitializer};
use crate::vk::command_buffer::{BarrierPhase, CommandBuffer};
use crate::vk::command_pool::CommandPool;
use crate::vk::descriptor::layout::DescriptorSetLayout;
use crate::vk::descriptor::set::DescriptorSetInfo;
use crate::vk::framebuffer::Framebuffer;
use crate::vk::image::{Image, ImageCreateInfo};
use crate::vk::logical_device::LogicalDevice;
use crate::vk::pipeline::layout::{PipelineLayout, PipelineLayoutInfo};
use crate::vk::pipeline::{DepthOptions, Pipeline, PipelineInfo};
use crate::vk::queue::Queue;
use crate::vk::render_pass::{RenderPass, SubpassAttachment};
use crate::vk::sampler::Sampler;

struct ViewportRendererState {
    width: u32,
    height: u32,
    render_pass: RenderPass,
    pipeline_layout: PipelineLayout,
    pipeline: Pipeline,
    swapchain_image_views: Vec<vk::ImageView>,
    framebuffers: Vec<Framebuffer>,
    buffers: Vec<(Buffer, Buffer)>,
    scale_factor: f32,
    physical_width: u32,
    physical_height: u32,
}
impl ViewportRendererState {
    pub fn get_vertex_buffer(&self, index : usize) -> &Buffer {
        &self.buffers[index].0
    }

    pub fn get_index_buffer(&self, index : usize) -> &Buffer {
        &self.buffers[index].1
    }
}

#[derive(Clone)]
struct ViewportRenderer {
    device: Arc<LogicalDevice>,
    descriptor_set_layout: Arc<DescriptorSetLayout>,
    state: Arc<Mutex<Option<ViewportRendererState>>>,
}
impl ViewportRenderer {
    fn new(device: Arc<LogicalDevice>, descriptor_set_layout: Arc<DescriptorSetLayout>) -> Self {
        Self {
            device,
            descriptor_set_layout,
            state: Arc::new(Mutex::new(None)),
        }
    }

    fn create_render_pass(device: &Arc<LogicalDevice>, surface_format: vk::Format) -> RenderPass {
        RenderPass::builder()
            .color_attachment(surface_format,
                vk::SampleCountFlags::TYPE_1,
                vk::AttachmentLoadOp::LOAD,
                vk::AttachmentStoreOp::STORE,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                vk::ImageLayout::PRESENT_SRC_KHR
            )
            .subpass(vk::PipelineBindPoint::GRAPHICS, &[
                SubpassAttachment::color(0)
            ], None)
            .dependency(vk::SUBPASS_EXTERNAL,
                0,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE
            )
            .build(device)
    }

    fn create_pipeline_layout(device: &Arc<LogicalDevice>, descriptor_set_layout: &Arc<DescriptorSetLayout>) -> PipelineLayout {
        PipelineLayoutInfo::default()
            .layout(descriptor_set_layout.as_ref())
            .push_constant(vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .offset(0)
                .size(size_of::<f32>() as u32 * 2))
            .build(device)
    }

    fn create_pipeline(device: &Arc<LogicalDevice>, render_pass: &RenderPass, pipeline_layout: &PipelineLayout) -> Pipeline {
        let attributes = [
            // position
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .offset(0)
                .location(0)
                .format(vk::Format::R32G32_SFLOAT),
            // uv
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .offset(8)
                .location(1)
                .format(vk::Format::R32G32_SFLOAT),
            // color
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .offset(16)
                .location(2)
                .format(vk::Format::R8G8B8A8_UNORM)
        ];

        PipelineInfo::default()
            // Topology
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            // Rasterization
            .samples(vk::SampleCountFlags::TYPE_1)
            // Shader modules
            .add_shader("./assets/gui.vert".into(), vk::ShaderStageFlags::VERTEX)
            .add_shader("./assets/gui.frag".into(), vk::ShaderStageFlags::FRAGMENT)
            // .polygon_mode(vk::PolygonMode::FILL)
            // Classify these
            .depth(DepthOptions::disabled()) // Might need some tuming
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .layout(pipeline_layout.handle())
            .render_pass(render_pass.handle())
            // TODO: fix blend
            .build(device)
    }

    fn create_framebuffers(
        device: &Arc<LogicalDevice>,
        swap_images: &[Image],
        render_pass: &RenderPass,
        width: u32,
        height: u32,
    ) -> (Vec<Framebuffer>, Vec<vk::ImageView>) {
        let swapchain_image_views = swap_images
            .iter()
            .map(|swapchain_image| swapchain_image.view())
            .collect::<Vec<_>>();
        let framebuffers = swapchain_image_views
            .iter()
            .map(|&image_view| {
                Framebuffer::new(device, vk::FramebufferCreateInfo::default()
                    .render_pass(render_pass.handle())
                    .attachments(&[image_view])
                    .width(width)
                    .height(height)
                    .layers(1)
                )
            })
            .collect::<Vec<_>>();

        (framebuffers, swapchain_image_views)
    }

    fn create_buffers(device: &Arc<LogicalDevice>, swapchain_count: usize,) -> Vec<(Buffer, Buffer)> { 
        let mut buffers = vec![];
        for _ in 0..swapchain_count {
            buffers.push((
                StaticBufferBuilder::fixed_size()
                    .cpu_to_gpu()
                    .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .build(device, Self::vertex_buffer_size()),
                StaticBufferBuilder::fixed_size()
                    .cpu_to_gpu()
                    .usage(vk::BufferUsageFlags::INDEX_BUFFER)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .build(device, Self::index_buffer_size())
            ));
        }
        buffers
    }

    fn update_swapchain(
        &mut self,
        width: u32,
        height: u32,
        swapchain_images: &[Image],
        surface_format: vk::Format,
        scale_factor: f32,
        physical_size: winit::dpi::PhysicalSize<u32>,
    ) {
        self.device.wait_idle();

        // cleanup framebuffers and others
        let (render_pass, pipeline_layout, pipeline) = {
            let Ok(mut state) = self.state.lock() else {
                panic!("Failed to lock state.");
            };
            if let Some(mut state) = state.take() {
                // Keep these, everything else drops - I think?
                (state.render_pass, state.pipeline_layout, state.pipeline)
            } else {
                let render_pass = Self::create_render_pass(&self.device, surface_format);
                let pipeline_layout = Self::create_pipeline_layout(&self.device, &self.descriptor_set_layout);
                let pipeline = Self::create_pipeline(&self.device, &render_pass, &pipeline_layout);
                (render_pass, pipeline_layout, pipeline)
            }
        };

        // Create Framebuffers
        let (framebuffers, swapchain_image_views) = Self::create_framebuffers(
            &self.device,
            swapchain_images,
            &render_pass,
            width,
            height,
        );

        let buffers = Self::create_buffers(&self.device, swapchain_images.len());

        // update self
        let mut state = self.state.lock().expect("Failed to lock state.");
        *state = Some(ViewportRendererState {
            width,
            height,
            render_pass,
            pipeline_layout,
            pipeline,
            swapchain_image_views,
            framebuffers,
            buffers,
            scale_factor,
            physical_width: physical_size.width,
            physical_height: physical_size.height,
        });
    }

    fn vertex_buffer_size() -> u64 { 1024 * 1024 * 4 }
    fn index_buffer_size() -> u64 { 1024 * 1024 * 4 }

    fn create_egui_cmd(
        &self,
        clipped_primitives: Vec<egui::ClippedPrimitive>,
        textures_delta: egui::TexturesDelta,
        managed_textures: Arc<Mutex<ManagedTextures>>,
        user_textures: Arc<Mutex<UserTextures>>,
        scale_factor: f32,
        physical_size: winit::dpi::PhysicalSize<u32>,
    ) -> EguiCommand {
        EguiCommand {
            swapchain_recreate_required: {
                let this = self.clone();
                let state = this.state.lock().unwrap();
                if let Some(state) = &*state {
                    state.scale_factor != scale_factor
                } else {
                    false
                }
            },
            swapchain_updater: Some(Box::new({
                let mut this = self.clone();
                move |swapchain_update_info| {
                    let SwapchainUpdateInfo {
                        width,
                        height,
                        swapchain_images,
                        surface_format,
                    } = swapchain_update_info;
                    this.update_swapchain(
                        width,
                        height,
                        &swapchain_images,
                        surface_format,
                        scale_factor,
                        physical_size
                    );
                }
            })),
            recorder: Box::new({
                let this = self.clone();
                move |cmd, index: usize| {
                    let state = this.state.lock().expect("Failed to lock state mutex.");
                    let state = state.as_ref().expect("State is none.");
                    let mut managed_textures =
                        managed_textures.lock().expect("Failed to lock textures.");
                    let mut user_textures =
                        user_textures.lock().expect("Failed to lock user textures.");

                    // update textures
                    managed_textures.update_textures(textures_delta);
                    user_textures.update_textures();

                    // get buffer ptr
                    let mut vertex_buffer_ptr = state.get_vertex_buffer(index).map();
                    let vertex_buffer_ptr_end =
                        unsafe { vertex_buffer_ptr.add(Self::vertex_buffer_size() as usize) };
                    let mut index_buffer_ptr = state.get_index_buffer(index).map();
                    let index_buffer_ptr_end =
                        unsafe { index_buffer_ptr.add(Self::index_buffer_size() as usize) };

                    // begin render pass
                    cmd.begin_render_pass(&state.render_pass, &state.framebuffers[index], vk::Rect2D::default()
                        .extent(vk::Extent2D::default()
                            .width(state.width)
                            .height(state.height),
                        ), &[], vk::SubpassContents::INLINE);

                    // bind resources
                    cmd.bind_pipeline(vk::PipelineBindPoint::GRAPHICS, &state.pipeline);
                    cmd.bind_vertex_buffers(0, &[(state.get_vertex_buffer(index), 0)]);
                    cmd.bind_index_buffer(state.get_index_buffer(index), 0);
                    {
                        let width_points = state.physical_width as f32 / state.scale_factor;
                        let height_points = state.physical_height as f32 / state.scale_factor;
                        cmd.push_constants(&state.pipeline, vk::ShaderStageFlags::VERTEX, 0, bytes_of(&width_points));
                        cmd.push_constants(&state.pipeline, vk::ShaderStageFlags::VERTEX, 4, bytes_of(&height_points));
                    }

                    // render meshes
                    let mut vertex_base = 0;
                    let mut index_base = 0;
                    for egui::ClippedPrimitive {
                        clip_rect,
                        primitive,
                    } in clipped_primitives
                    {
                        let mesh = match primitive {
                            egui::epaint::Primitive::Mesh(mesh) => mesh,
                            egui::epaint::Primitive::Callback(_) => todo!(),
                        };
                        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
                            continue;
                        }

                        unsafe {
                            match mesh.texture_id {
                                egui::TextureId::User(id) => {
                                    if let Some(&descriptor_set) =
                                        user_textures.texture_desc_sets.get(&id)
                                    {
                                        cmd.bind_descriptor_sets(vk::PipelineBindPoint::GRAPHICS, &state.pipeline, 0, &[descriptor_set], &[]);
                                    } else {
                                        // DO nothing, orphaned texture
                                        continue;
                                    }
                                }
                                egui::TextureId::Managed(_) => {
                                    cmd.bind_descriptor_sets(vk::PipelineBindPoint::GRAPHICS, &state.pipeline, 0, &[*managed_textures
                                        .texture_desc_sets
                                        .get(&mesh.texture_id)
                                        .unwrap()], &[]);
                                }
                            }
                        }
                        let v_slice = &mesh.vertices;
                        let v_size = std::mem::size_of::<egui::epaint::Vertex>();
                        let v_copy_size = v_slice.len() * v_size;

                        let i_slice = &mesh.indices;
                        let i_size = std::mem::size_of::<u32>();
                        let i_copy_size = i_slice.len() * i_size;

                        let vertex_buffer_ptr_next = unsafe { vertex_buffer_ptr.add(v_copy_size) };
                        let index_buffer_ptr_next = unsafe { index_buffer_ptr.add(i_copy_size) };

                        if vertex_buffer_ptr_next >= vertex_buffer_ptr_end
                            || index_buffer_ptr_next >= index_buffer_ptr_end
                        {
                            panic!("egui paint out of memory");
                        }

                        // map memory
                        unsafe {
                            vertex_buffer_ptr.copy_from(v_slice.as_ptr() as *const u8, v_copy_size)
                        };
                        unsafe {
                            index_buffer_ptr.copy_from(i_slice.as_ptr() as *const u8, i_copy_size)
                        };

                        vertex_buffer_ptr = vertex_buffer_ptr_next;
                        index_buffer_ptr = index_buffer_ptr_next;

                        // record draw commands
                        unsafe {
                            let min = clip_rect.min;
                            let min = egui::Pos2 {
                                x: min.x * state.scale_factor as f32,
                                y: min.y * state.scale_factor as f32,
                            };
                            let min = egui::Pos2 {
                                x: f32::clamp(min.x, 0.0, state.physical_width as f32),
                                y: f32::clamp(min.y, 0.0, state.physical_height as f32),
                            };
                            let max = clip_rect.max;
                            let max = egui::Pos2 {
                                x: max.x * state.scale_factor as f32,
                                y: max.y * state.scale_factor as f32,
                            };
                            let max = egui::Pos2 {
                                x: f32::clamp(max.x, min.x, state.physical_width as f32),
                                y: f32::clamp(max.y, min.y, state.physical_height as f32),
                            };
                            cmd.set_scissors(0, &[
                                vk::Rect2D::default()
                                    .offset(vk::Offset2D {
                                        x: min.x.round() as i32,
                                        y: min.y.round() as i32,
                                    })
                                    .extent(vk::Extent2D {
                                        width: (max.x.round() - min.x) as u32,
                                        height: (max.y.round() - min.y) as u32,
                                    })
                            ]);
                            cmd.set_viewport(0, &[
                                vk::Viewport::default()
                                    .x(0.0)
                                    .y(0.0)
                                    .width(state.physical_width as f32)
                                    .height(state.physical_height as f32)
                                    .min_depth(0.0)
                                    .max_depth(1.0)
                            ]);
                            cmd.draw_indexed(mesh.indices.len() as _, 1, index_base, vertex_base, 0);
                        }

                        vertex_base += mesh.vertices.len() as i32;
                        index_base += mesh.indices.len() as u32;
                    }

                    cmd.end_render_pass();
                }
            }),
        }
    }
}

struct ManagedTextures {
    device: Arc<LogicalDevice>,
    queue: Queue,
    descriptor_set_layout: Arc<DescriptorSetLayout>,
    sampler: Sampler,

    transfer_command_pool : CommandPool,
    transfer_fence : vk::Fence,

    texture_desc_sets: HashMap<egui::TextureId, vk::DescriptorSet>,
    texture_images: HashMap<egui::TextureId, Image>,
}

impl ManagedTextures {
    fn new(device: Arc<LogicalDevice>, queue: Queue, descriptor_set_layout : Arc<DescriptorSetLayout>) -> Arc<Mutex<Self>> {
        let sampler = Sampler::builder()
            .address_mode(vk::SamplerAddressMode::CLAMP_TO_EDGE, vk::SamplerAddressMode::CLAMP_TO_EDGE, vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .anisotropy(false)
            .filter(vk::Filter::LINEAR, vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .lod(0.0, vk::LOD_CLAMP_NONE)
            .build(&device);

        Arc::new(Mutex::new(Self {
            transfer_command_pool : CommandPool::builder(queue.family()).build(&device),
            transfer_fence : device.create_fence(vk::FenceCreateFlags::empty()),

            device,
            queue,
            descriptor_set_layout,
            sampler,
            texture_desc_sets: HashMap::new(),
            texture_images: HashMap::new(),
        }))
    }

    fn update_texture(&mut self, texture_id: egui::TextureId, delta: egui::epaint::ImageDelta) {
        // Extract pixel data from egui
        let data: Vec<u8> = match &delta.image {
            egui::ImageData::Color(image) => {
                assert_eq!(
                    image.width() * image.height(),
                    image.pixels.len(),
                    "Mismatch between texture size and texel count"
                );

                image.pixels.iter()
                    .flat_map(|color| color.to_array())
                    .collect()
            }
            egui::ImageData::Font(image) => image
                .srgba_pixels(None)
                .flat_map(|color| color.to_array())
                .collect(),
        };

        let cmd = CommandBuffer::builder()
            .pool(&self.transfer_command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .build_one(&self.device);

        let mut staging_buffer = StaticBufferBuilder::fixed_size()
            .name("GUI/Image Staging Buffer")
            .cpu_to_gpu()
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .build(&self.device, data.len() as u64);

        staging_buffer.update(&data);
        
        let mut texture_image = ImageCreateInfo::default()
            .color()
            .extent(vk::Extent3D {
                width: delta.image.width() as u32,
                height: delta.image.height() as u32,
                depth: 1,
            })
            .image_type(vk::ImageType::TYPE_2D, vk::ImageViewType::TYPE_2D)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .samples(vk::SampleCountFlags::TYPE_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::TRANSFER_SRC)
            .format(vk::Format::R8G8B8A8_UNORM)
            .layers(0, 1)
            .levels(0, 1)
            .build(&self.device);

        let texture_image_view = texture_image.view();

        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        cmd.image_memory_barrier(&mut texture_image, 
            BarrierPhase(vk::QUEUE_FAMILY_IGNORED, vk::AccessFlags::NONE_KHR,       vk::PipelineStageFlags::HOST),
            BarrierPhase(vk::QUEUE_FAMILY_IGNORED, vk::AccessFlags::TRANSFER_WRITE, vk::PipelineStageFlags::TRANSFER),
            vk::DependencyFlags::BY_REGION,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL);
        cmd.copy_buffer_to_image(&staging_buffer, &texture_image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &[
            vk::BufferImageCopy::default()
                .buffer_offset(0)
                .buffer_row_length(delta.image.width() as u32)
                .buffer_image_height(delta.image.height() as u32)
                .image_subresource(texture_image.make_subresource_layer(0, None, None))
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(vk::Extent3D {
                    width: delta.image.width() as u32,
                    height: delta.image.height() as u32,
                    depth: 1,
                }),
        ]);
        cmd.image_memory_barrier(&mut texture_image, 
            BarrierPhase(vk::QUEUE_FAMILY_IGNORED, vk::AccessFlags::TRANSFER_WRITE, vk::PipelineStageFlags::TRANSFER),
            BarrierPhase(vk::QUEUE_FAMILY_IGNORED, vk::AccessFlags::SHADER_READ,    vk::PipelineStageFlags::VERTEX_SHADER),
            vk::DependencyFlags::BY_REGION,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        cmd.end();
        
        self.device.submit(&self.queue, &[&cmd], &[], &[], self.transfer_fence);
        self.device.wait_for_fence(self.transfer_fence);

        // texture is now in GPU memory, now we need to decide whether we should register it as new or update existing

        if let Some(pos) = delta.pos {
            // Blit texture data to existing texture if delta pos exists (e.g. font changed)
            let existing_texture = self.texture_images.get_mut(&texture_id);
            if let Some(existing_texture) = existing_texture {
                let extent = vk::Extent3D {
                    width: delta.image.width() as u32,
                    height: delta.image.height() as u32,
                    depth: 1,
                };
                unsafe {
                    self.transfer_command_pool.reset(vk::CommandPoolResetFlags::empty());
                    self.device.reset_fences(&[self.transfer_fence]);

                    cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
                    cmd.image_memory_barrier(existing_texture, // Update the target image to a TRANSFER_DST layout
                        BarrierPhase(vk::QUEUE_FAMILY_IGNORED, vk::AccessFlags::SHADER_READ,    vk::PipelineStageFlags::FRAGMENT_SHADER),
                        BarrierPhase(vk::QUEUE_FAMILY_IGNORED, vk::AccessFlags::TRANSFER_WRITE, vk::PipelineStageFlags::TRANSFER),
                        vk::DependencyFlags::BY_REGION,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL
                    );    
                    cmd.image_memory_barrier(&mut texture_image, // And the source to a TRANSFER_SRC layout
                        BarrierPhase(vk::QUEUE_FAMILY_IGNORED, vk::AccessFlags::SHADER_READ,   vk::PipelineStageFlags::FRAGMENT_SHADER),
                        BarrierPhase(vk::QUEUE_FAMILY_IGNORED, vk::AccessFlags::TRANSFER_READ, vk::PipelineStageFlags::TRANSFER),
                        vk::DependencyFlags::BY_REGION,
                        vk::ImageLayout::TRANSFER_SRC_OPTIMAL
                    );

                    let top_left = vk::Offset3D {
                        x: pos[0] as i32,
                        y: pos[1] as i32,
                        z: 0,
                    };
                    let bottom_right = vk::Offset3D {
                        x: pos[0] as i32 + delta.image.width() as i32,
                        y: pos[1] as i32 + delta.image.height() as i32,
                        z: 1,
                    };

                    let region = vk::ImageBlit {
                        src_subresource: texture_image.make_subresource_layer(0, None, None),
                        src_offsets: [
                            vk::Offset3D { x: 0, y: 0, z: 0 },
                            vk::Offset3D {
                                x: extent.width as i32,
                                y: extent.height as i32,
                                z: extent.depth as i32,
                            },
                        ],
                        dst_subresource: existing_texture.make_subresource_layer(0, None, None),
                        dst_offsets: [top_left, bottom_right],
                    };
                    cmd.blit_image(&texture_image, existing_texture, &[region], vk::Filter::NEAREST);

                    cmd.image_memory_barrier(existing_texture, 
                        BarrierPhase(vk::QUEUE_FAMILY_IGNORED, vk::AccessFlags::TRANSFER_WRITE, vk::PipelineStageFlags::TRANSFER),
                        BarrierPhase(vk::QUEUE_FAMILY_IGNORED, vk::AccessFlags::SHADER_READ,    vk::PipelineStageFlags::FRAGMENT_SHADER),
                        vk::DependencyFlags::BY_REGION,
                        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
                    cmd.end();
                    
                    self.device.submit(&self.queue, &[&cmd], &[], &[], self.transfer_fence);
                    self.device.wait_for_fence(self.transfer_fence);
                }
            } else {
                return;
            }
        } else {
            // Otherwise save the newly created texture
            let dsc_set = self.descriptor_set_layout.request(DescriptorSetInfo::default()
                .images(0, vec![
                    vk::DescriptorImageInfo {
                        sampler: self.sampler.handle(),
                        image_view: texture_image_view,
                        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
                    }
                ])
            );

            // Replace the old texture; this drops the associated Vulkan resources.
            self.texture_images.insert(texture_id, texture_image);
            self.texture_desc_sets.insert(texture_id, dsc_set);
        }
    }

    fn free_texture(&mut self, id : egui::TextureId) {
        self.texture_desc_sets.remove_entry(&id);
    }

    fn update_textures(&mut self, textures_delta: egui::TexturesDelta) {
        for (id, image_delta) in textures_delta.set {
            self.update_texture(id, image_delta);
        }
        for id in textures_delta.free {
            self.free_texture(id);
        }
    }
}

pub(crate) type ImageRegistryReceiver = Receiver<RegistryCommand>;

#[derive(Clone)]
pub struct ImageRegistry {
    sender: Sender<RegistryCommand>,
    counter: Arc<AtomicU64>,
}
impl ImageRegistry {
    pub(crate) fn new() -> (Self, ImageRegistryReceiver) {
        let (sender, receiver) = mpsc::channel();
        (
            Self {
                sender,
                counter: Arc::new(AtomicU64::new(0)),
            },
            receiver,
        )
    }

    pub fn register_user_texture(&self, image_view: vk::ImageView, sampler: &Sampler) -> egui::TextureId {
        let id = egui::TextureId::User(self.counter.fetch_add(1, Ordering::SeqCst));
        self.sender
            .send(RegistryCommand::RegisterUserTexture {
                image_view,
                sampler : sampler.handle(),
                id,
            })
            .expect("Failed to send register user texture command.");
        id
    }

    pub fn unregister_user_texture(&self, id: egui::TextureId) {
        let _ = self
            .sender
            .send(RegistryCommand::UnregisterUserTexture { id });
    }
}
impl Debug for ImageRegistry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageRegistry").finish()
    }
}

pub(crate) enum RegistryCommand {
    RegisterUserTexture {
        image_view: vk::ImageView,
        sampler: vk::Sampler,
        id: egui::TextureId,
    },
    UnregisterUserTexture {
        id: egui::TextureId,
    },
}

struct UserTextures {
    device: Arc<LogicalDevice>,
    descriptor_set_layout: Arc<DescriptorSetLayout>,
    texture_desc_sets: HashMap<u64, vk::DescriptorSet>,
    receiver: ImageRegistryReceiver,
}
impl UserTextures {
    fn new(
        device: Arc<LogicalDevice>,
        descriptor_set_layout: Arc<DescriptorSetLayout>,
        receiver: ImageRegistryReceiver,
    ) -> Arc<Mutex<Self>> {
        let texture_desc_sets = HashMap::new();

        Arc::new(Mutex::new(Self {
            device,
            descriptor_set_layout,
            texture_desc_sets,
            receiver,
        }))
    }

    fn register_user_texture(&mut self, id: u64, image_view: vk::ImageView, sampler: vk::Sampler) {
        let dsc_set = self.descriptor_set_layout.request(DescriptorSetInfo::default()
            .images(0, vec![
                vk::DescriptorImageInfo::default()
                    .sampler(sampler)
                    .image_view(image_view)
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            ])
        );
    }

    fn unregister_user_texture(&mut self, id: u64) {
        if let Some(desc_set) = self.texture_desc_sets.remove(&id) {
            self.descriptor_set_layout.forget(desc_set);
        }
    }

    fn update_textures(&mut self) {
        for command in self.receiver.try_iter().collect::<Vec<_>>() {
            match command {
                RegistryCommand::RegisterUserTexture {
                    image_view,
                    sampler,
                    id,
                } => match id {
                    egui::TextureId::Managed(_) => panic!("This texture id is not for user texture: {:?}", id),
                    egui::TextureId::User(id) => self.register_user_texture(id, image_view, sampler),
                },
                RegistryCommand::UnregisterUserTexture { id } => match id {
                    egui::TextureId::Managed(_) => panic!("This texture id is not for user texture: {:?}", id),
                    egui::TextureId::User(id) => self.unregister_user_texture(id),
                },
            }
        }
    }
}

pub(crate) struct Renderer {
    device: Arc<LogicalDevice>,
    descriptor_set_layout: Arc<DescriptorSetLayout>,
    viewport_renderers: HashMap<egui::ViewportId, ViewportRenderer>,

    managed_textures: Arc<Mutex<ManagedTextures>>,
    user_textures: Arc<Mutex<UserTextures>>,
}
impl Renderer {
    fn create_descriptor_set_layout(device: &Arc<LogicalDevice>) -> Arc<DescriptorSetLayout> {
        DescriptorSetLayout::builder()
            .sets(1024)
            .pool_flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .binding(0, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT, 1)
            .build(device)
    }

    pub(crate) fn new(
        device: &Arc<LogicalDevice>,
        queue: Queue,
        receiver: Receiver<RegistryCommand>,
    ) -> Arc<Mutex<Self>> {
        let descriptor_set_layout = Self::create_descriptor_set_layout(&device);
        Arc::new(Mutex::new(Self {
            device: device.clone(),
            descriptor_set_layout,
            viewport_renderers: HashMap::new(),
            managed_textures: ManagedTextures::new(device.clone(), queue, descriptor_set_layout.clone()),
            user_textures: UserTextures::new(device.clone(), descriptor_set_layout, receiver),
        }))
    }

    pub(crate) fn create_egui_cmd(
        &mut self,
        viewport_id: egui::ViewportId,
        clipped_primitives: Vec<egui::ClippedPrimitive>,
        textures_delta: egui::TexturesDelta,
        scale_factor: f32,
        physical_size: winit::dpi::PhysicalSize<u32>,
    ) -> EguiCommand {
        let viewport_renderer = self
            .viewport_renderers
            .entry(viewport_id)
            .or_insert_with(|| {
                ViewportRenderer::new(self.device.clone(), self.descriptor_set_layout)
            });
        viewport_renderer.create_egui_cmd(
            clipped_primitives,
            textures_delta,
            self.managed_textures.clone(),
            self.user_textures.clone(),
            scale_factor,
            physical_size,
        )
    }

    pub(crate) fn destroy_viewports(&mut self, active_viewport_ids: &egui::ViewportIdSet) {
        let remove_viewports = self
            .viewport_renderers
            .keys()
            .filter(|id| !active_viewport_ids.contains(id))
            .filter(|id| id != &&egui::ViewportId::ROOT)
            .map(|id| id.clone())
            .collect::<Vec<_>>();

        for id in remove_viewports {
            self.viewport_renderers.remove(&id);
        }
    }
}

/// struct to pass to `EguiCommand::update_swapchain` method.
pub struct SwapchainUpdateInfo {
    pub width: u32,
    pub height: u32,
    pub swapchain_images: Vec<Image>,
    pub surface_format: vk::Format,
}

/// command recorder to record egui draw commands.
///
/// if you recreate swapchain, you must call `update_swapchain` method.
/// You also must call `update_swapchain` method when first time to record commands.
pub struct EguiCommand {
    swapchain_updater: Option<Box<dyn FnOnce(SwapchainUpdateInfo) + Send>>,
    recorder: Box<dyn FnOnce(&CommandBuffer, usize) + Send>,
    swapchain_recreate_required: bool,
}
impl EguiCommand {
    /// You must call this method once when first time to record commands
    /// and when you recreate swapchain.
    pub fn update_swapchain(&mut self, info: SwapchainUpdateInfo) {
        (self.swapchain_updater.take().expect(
            "The swapchain has been updated more than once. Never update the swapchain more than once.",
        ))(info);
    }

    /// record commands to command buffer.
    pub fn record(self, cmd: &CommandBuffer, swapchain_index: usize) {
        (self.recorder)(cmd, swapchain_index);
    }

    /// Returns whether swapchain recreation is required.
    pub fn swapchain_recreate_required(&self) -> bool {
        self.swapchain_recreate_required
    }
}
impl Default for EguiCommand {
    fn default() -> Self {
        Self {
            swapchain_updater: None,
            recorder: Box::new(|_, _| {}),
            swapchain_recreate_required: false,
        }
    }
}
