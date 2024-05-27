use std::collections::HashMap;
use std::mem::size_of;
use std::sync::Arc;
use ash::vk::{self};
use egui::epaint::ImageDelta;
use egui::{Color32, Context, FontDefinitions, Style, TextureId, TexturesDelta, ViewportId};
use raw_window_handle::HasDisplayHandle;
use crate::traits::handle::Handle;
use crate::vk::buffer::{Buffer, DynamicBufferBuilder, DynamicInitializer, StaticBufferBuilder, StaticInitializer};
use crate::vk::command_buffer::{BarrierPhase, CommandBuffer};
use crate::vk::descriptor::layout::{BindingDescriptorCount, DescriptorSetLayout, DescriptorSetLayoutBuilder, PoolDescriptorCount};
use crate::vk::descriptor::set::DescriptorSetInfo;
use crate::vk::framebuffer::Framebuffer;
use crate::vk::helpers::{prepare_buffer_image_copy, with_delta};
use crate::vk::image::{Image, ImageCreateInfo};
use crate::vk::logical_device::LogicalDevice;
use crate::vk::pipeline::layout::PipelineLayoutInfo;
use crate::vk::pipeline::{DepthOptions, Pipeline, PipelineInfo, Vertex};
use crate::vk::queue::{Queue, QueueAffinity};
use crate::vk::render_pass::{RenderPass, SubpassAttachment};
use crate::vk::renderer::Renderer;
use crate::vk::sampler::Sampler;
use crate::window::Window;

// A GUI texture.
struct Texture {
    image : Image,
    descriptor_set : DescriptorSetInfo
}

pub struct Interface {
    context : Context,
    egui : egui_winit::State,

    device : Arc<LogicalDevice>,
    render_pass : RenderPass,
    pipeline : Pipeline,
    framebuffers : Vec<Framebuffer>,
    descriptor_set_layout : DescriptorSetLayout,

    extent : vk::Extent2D,

    vertex_buffers : Vec<Buffer>,
    index_buffers : Vec<Buffer>,

    textures : HashMap<TextureId, Texture>,
}

struct InterfaceVertex;

impl Vertex for InterfaceVertex {
    // https://github.com/MatchaChoco010/egui-winit-ash-integration/blob/main/src/integration.rs#L179
    fn bindings() -> Vec<(u32, vk::VertexInputRate)> {
        vec![
            (4 * (size_of::<f32>() + size_of::<u8>()) as u32, vk::VertexInputRate::VERTEX)
        ]
    }

    // https://github.com/MatchaChoco010/egui-winit-ash-integration/blob/main/src/integration.rs#L179
    fn format_offset() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription::default() // Position
                .binding(0)
                .offset(0)
                .location(0)
                .format(vk::Format::R32G32_SFLOAT),
            vk::VertexInputAttributeDescription::default() // UV
                .binding(0)
                .offset(8)
                .location(1)
                .format(vk::Format::R32G32_SFLOAT),
            vk::VertexInputAttributeDescription::default() // Color
                .binding(0)
                .offset(16)
                .location(2)
                .format(vk::Format::R8G8B8A8_UNORM)
        ]
    }
}

struct InterfaceCreateInfo<'a> {
    fonts : FontDefinitions,
    style : Style,
    pixel_per_point : f32,

    renderer : &'a Renderer,

    // GUI-specific pipeline stuff
    render_pass : RenderPass,
    descriptor_set_layout : DescriptorSetLayout,
    pipeline : Pipeline,

    framebuffers : Vec<Framebuffer>,
    vertex_buffers : Vec<Buffer>,
    index_buffers : Vec<Buffer>,
    sampler : Sampler,
}

impl Interface {
    pub(in crate) fn new<H>(
        info : InterfaceCreateInfo,
        target : &H
    ) -> Self
        where H : HasDisplayHandle
    {
        let context = Context::default();
        context.set_fonts(info.fonts);
        context.set_style(info.style);

        let mut egui = egui_winit::State::new(context.clone(),
            ViewportId::ROOT,
            target,
            Some(info.pixel_per_point),
            None);

        // Create a render pass.
        let render_pass = RenderPass::builder()
            .color_attachment(
                info.renderer.swapchain.surface_format.format,
                vk::SampleCountFlags::TYPE_1,
                vk::AttachmentLoadOp::LOAD,
                vk::AttachmentStoreOp::STORE,
                vk::ImageLayout::PRESENT_SRC_KHR
            )
            .dependency(
                vk::SUBPASS_EXTERNAL,
                0,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE
            )
            .subpass(vk::PipelineBindPoint::GRAPHICS, &[
                SubpassAttachment::color(0)
            ], SubpassAttachment::None)
            .build(&info.renderer.device);

        // Create a descriptor pool.
        let descriptor_set_layout = DescriptorSetLayoutBuilder::default()
            .sets(1024)
            .binding(0, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT, PoolDescriptorCount(1024), BindingDescriptorCount(1))
            .build(&info.renderer);

        let pipeline_layout = PipelineLayoutInfo::default()
            .layout(&descriptor_set_layout)
            .push_constant(vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .offset(0)
                .size(size_of::<f32>() as u32 * 2) // Screen size
            )
            .build(&info.renderer);

        let pipeline = PipelineInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .layout(pipeline_layout.handle())
            .depth(DepthOptions::disabled())
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .render_pass(vk::RenderPass::null())
            .samples(vk::SampleCountFlags::TYPE_1)
            .pool(&info.renderer.pipeline_cache)
            .vertex::<InterfaceVertex>()
            .add_shader("./assets/gui.frag".into(), vk::ShaderStageFlags::VERTEX)
            .add_shader("./assets/gui.frag".into(), vk::ShaderStageFlags::FRAGMENT)
            .build(&info.renderer.device);

        let sampler = Sampler::builder()
            .address_mode(vk::SamplerAddressMode::CLAMP_TO_EDGE, vk::SamplerAddressMode::CLAMP_TO_EDGE, vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .anisotropy(false)
            .filter(vk::Filter::LINEAR, vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .lod(0.0, vk::LOD_CLAMP_NONE)
            .build(&info.renderer.device);

        let image_views = info.renderer.swapchain.present_images.iter()
            .map(Image::view)
            .collect::<Vec<_>>();

        let framebuffers = info.renderer.swapchain.create_framebuffers(&render_pass);
        let mut vertex_buffers = vec![];
        let mut index_buffers = vec![];
        // let mut update_buffers = vec![];
        for i in 0..framebuffers.len() {
            vertex_buffers.push(StaticBufferBuilder::fixed_size()
                .cpu_to_gpu()
                .linear(true)
                .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
                .index(vk::IndexType::UINT16)
                .build(&info.renderer, 1024 * 1024 * 4)
            );

            index_buffers.push(StaticBufferBuilder::fixed_size()
                .cpu_to_gpu()
                .linear(true)
                .usage(vk::BufferUsageFlags::INDEX_BUFFER)
                .build(&info.renderer, 1024 * 1024 * 2)
            );
        }

        Self {
            context,
            egui,
            render_pass,
            descriptor_set_layout,
            pipeline,
            framebuffers,
            vertex_buffers,
            index_buffers,

            extent : info.renderer.swapchain.extent,
            device : info.renderer.device.clone(),

            textures : HashMap::default(),
        }
    }

    pub fn begin_frame(&mut self, window : &Window) {
        let raw_input = self.egui.take_egui_input(window.handle());
        self.context.begin_frame(raw_input);
    }

    pub fn end_frame(&mut self, window : &Window) -> egui::FullOutput {
        let output = self.context.end_frame();
        self.egui.handle_platform_output(window.handle(), output.platform_output.clone());

        output
    }

    pub fn paint(&mut self,
        cmd : &CommandBuffer,
        swapchain_image_index : usize,
        clipped_meshes : Vec<egui::ClippedPrimitive>,
        texture_delta : TexturesDelta
    ) {
        /*for (id, image_delta) in texture_delta.set {
            self.update_texture(id, image_delta);
        }

        let mut vertex_buffer = self.vertex_buffers[swapchain_image_index]
            .map();
        let mut index_buffer = self.index_buffers[swapchain_image_index]
            .map();

        cmd.begin_render_pass(
            &self.render_pass,
            &self.framebuffers[swapchain_image_index], 
            vk::Rect2D::default()
                .offset(vk::Offset2D::default().x(0).y(0))
                .extent(self.extent),
            &[],
            vk::SubpassContents::INLINE
        );

        cmd.bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline)
        cmd.bind_vertex_buffers(0,
            &[&self.vertex_buffers[swapchain_image_index]],
            &[0]
        );
        cmd.bind_index_buffers(0,
            &[&self.index_buffers[swapchain_image_index]],
            &[0]
        );
        cmd.set_viewport(0, &[
            vk::Viewport::default()
                .x(0.0)
                .y(0.0)
                .min_depth(0.0)
                .max_depth(1.0)
                .width(self.extent.width as f32)
                .height(self.extent.height as f32)
        ]);

        let width_points = self.extent.width as f32 / self.scale_factor as f32;
        let height_points = self.extent.height as f32 / self.scale_factor as f32;

        cmd.push_constants(&self.pipeline_layout,
            vk::ShaderStageFlags::VERTEX,
            0,
            bytes_of(&width_points)
        );
        cmd.push_constants(&self.pipeline_layout,
            vk::ShaderStageFlags::VERTEX,
            size_of_val(width_points) as u32,
            bytes_of(&height_points)
        );

        // Render the meshes
        let mut vertex_base = 0;
        let mut index_base = 0;
        for egui::ClippedPrimitive { clip_rect, primitive } in clipped_meshes {
            let mesh = match primitive {
                Primitive::Mesh(mesh) => mesh,
                Primitive::Callback(_) => todo!(),
            };

            if mesh.is_empty() {
                continue;
            }

            if let egui::TextureId::User(user) = mesh.texture_id {
                if let Some(descriptor) = self.user_textures[id as usize] {
                    cmd.bind_descriptor_sets(
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline_layout,
                        0,
                        &[descriptor_set],
                        &[]
                    );
                } else {
                    continue;
                }
            } else {
                cmd.bind_descriptor_sets(
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline_layout,
                    0,
                    &[*self.texture_desc_sets.get(&mesh.texture_id).unwrap()],
                    &[]
                );
            }

            let v_slice = &mesh.vertices;
            let v_size = size_of_val(&v_slice[0]);
            let v_copy_size = v_slice.len() * v_size;

            let i_slice = &mesh.indices;
            let i_size = size_of_val(&i_slice[0]);
            let i_copy_size = i_slice.len() * i_size;
        }*/
    }
    
    fn update_texture(&mut self, renderer : &Renderer, tex_id : TextureId, delta : ImageDelta) {
        let data = match &delta.image {
            egui::ImageData::Color(color) => color.pixels.iter().flat_map(Color32::to_array).collect::<Vec<_>>(),
            egui::ImageData::Font(font) => font.srgba_pixels(None).flat_map(|c| c.to_array()).collect(),
        };

        // Create a fence
        let fence = self.device.create_fence(vk::FenceCreateFlags::empty());
        let transfer_queue : &Queue = self.device.get_queues(QueueAffinity::Transfer).get(0).expect("Could not find transfer queue");

        // Allocate a buffer for the data.
        let transfer_src = DynamicBufferBuilder::dynamic()
            .cpu_to_gpu()
            .linear(true)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .build(&renderer, &data);

        let mut image = ImageCreateInfo::default()
            .color()
            .layers(0, 1)
            .levels(0, 1)
            .image_type(vk::ImageType::TYPE_2D, vk::ImageViewType::TYPE_2D)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::TRANSFER_SRC)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .name("GUI staging texture".to_owned())
            .format(vk::Format::R8G8B8A8_UNORM)
            .build(&renderer.device);

        let cmd = CommandBuffer::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .pool(&renderer.transfer_pool)
            .build_one(&renderer.device);

        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        // Transition the new image to transfer dest
        cmd.image_memory_barrier(&mut image,
            BarrierPhase::ignore_queue(vk::AccessFlags::NONE_KHR,       vk::PipelineStageFlags::HOST),
            BarrierPhase::ignore_queue(vk::AccessFlags::TRANSFER_WRITE, vk::PipelineStageFlags::TRANSFER),
            vk::DependencyFlags::BY_REGION,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL
        );
        cmd.copy_buffer_to_image(&transfer_src, &image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &[
            // TODO: This is kind of obscure, clean this up. The amount of lines of code Vulkan
            //       forces me to write here is a bit insane.
            with_delta(&delta, prepare_buffer_image_copy(&image, 0))
        ]);
        // Transition the new image to shader src
        cmd.image_memory_barrier(&mut image,
            BarrierPhase::ignore_queue(vk::AccessFlags::TRANSFER_WRITE, vk::PipelineStageFlags::TRANSFER),
            BarrierPhase::ignore_queue(vk::AccessFlags::SHADER_READ,    vk::PipelineStageFlags::VERTEX_SHADER),
            vk::DependencyFlags::BY_REGION,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        );
        cmd.end();

        renderer.device.submit(transfer_queue, &[&cmd], &[], &[], fence);
        renderer.device.wait_for_fence(fence);

        // The texture now lives in GPU memory, so we should decide if it has to be registered as a new texture, or update an existing one
        if let Some(pos) = delta.pos {
            // Blit texture data to the existing texture if delta pos exists (which can happen if a font changes)
            let existing_texture = self.textures.get_mut(&tex_id);
            if let Some(existing_texture) = existing_texture {
                renderer.reset_fence(fence);

                cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT); // Reuse this command buffer

                // Transition the existing image to transfer dst
                cmd.image_memory_barrier(&mut existing_texture.image,
                    BarrierPhase::ignore_queue(vk::AccessFlags::SHADER_READ,    vk::PipelineStageFlags::FRAGMENT_SHADER),
                    BarrierPhase::ignore_queue(vk::AccessFlags::TRANSFER_WRITE, vk::PipelineStageFlags::TRANSFER),
                    vk::DependencyFlags::BY_REGION,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL
                );
                // Transition the new image to transfer SRC
                cmd.image_memory_barrier(&mut image,
                    BarrierPhase::ignore_queue(vk::AccessFlags::SHADER_READ,   vk::PipelineStageFlags::FRAGMENT_SHADER),
                    BarrierPhase::ignore_queue(vk::AccessFlags::TRANSFER_READ, vk::PipelineStageFlags::TRANSFER),
                    vk::DependencyFlags::BY_REGION,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL
                );
                let dst_subresource = existing_texture.image.make_subresource_layer(0);
                cmd.blit_image(&image,
                    &mut existing_texture.image,
                    &[
                        vk::ImageBlit::default()
                            .src_subresource(image.make_subresource_layer(0))
                            .src_offsets([
                                vk::Offset3D { x: 0, y: 0, z: 0 },
                                vk::Offset3D {
                                    x: image.extent().width as i32,
                                    y: image.extent().height as i32,
                                    z: image.extent().depth as i32,
                                },
                            ])
                            .dst_subresource(dst_subresource)
                            .dst_offsets([
                                vk::Offset3D { x : pos[0] as i32, y : pos[1] as i32, z : 0},
                                vk::Offset3D {
                                    x : pos[0] as i32 + delta.image.width() as i32,
                                    y : pos[1] as i32 + delta.image.height() as i32,
                                    z : 1,
                                }
                            ])
                    ],
                    vk::Filter::NEAREST
                );

                // Transition the existing image to shader source
                cmd.image_memory_barrier(&mut existing_texture.image,
                    BarrierPhase::ignore_queue(vk::AccessFlags::TRANSFER_WRITE, vk::PipelineStageFlags::TRANSFER),
                    BarrierPhase::ignore_queue(vk::AccessFlags::SHADER_READ,    vk::PipelineStageFlags::FRAGMENT_SHADER),
                    vk::DependencyFlags::BY_REGION,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
                );
                cmd.end();

                renderer.device.submit(transfer_queue, &[&cmd], &[], &[], fence);
                renderer.device.wait_for_fence(fence);

                // The new image gets dropped here.
            } else {
                // ??? What's going on ???
            }
        } else {
            self.textures.insert(tex_id, Texture {
                image,
                descriptor_set : todo!("Descriptor set")
            });
        }
    }
}