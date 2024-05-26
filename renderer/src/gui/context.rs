use std::mem::{size_of, size_of_val};
use std::sync::Arc;
use ash::vk::{self, PipelineLayout};
use egui::{Context, FontDefinitions, Style, TexturesDelta, ViewportId};
use raw_window_handle::HasDisplayHandle;
use crate::traits::handle::Handle;
use crate::vk::buffer::Buffer;
use crate::vk::command_buffer::CommandBuffer;
use crate::vk::descriptor::layout::{BindingDescriptorCount, DescriptorSetLayoutBuilder, PoolDescriptorCount};
use crate::vk::framebuffer::Framebuffer;
use crate::vk::pipeline::layout::PipelineLayoutInfo;
use crate::vk::pipeline::{DepthOptions, Pipeline, PipelineInfo, Vertex};
use crate::vk::render_pass::{RenderPass, RenderPassCreateInfo};
use crate::vk::renderer::Renderer;
use crate::window::Window;

pub struct Interface {
    context : Context,
    egui : egui_winit::State,

    render_pass : RenderPass,
    pipeline : Pipeline,
    framebuffers : Vec<Framebuffer>,

    extent : vk::Extent2D,

    vertex_buffers : Vec<Buffer>,
    index_buffers : Vec<Buffer>,
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
    fn format_offset() -> Vec<(vk::Format, u32)> {
        vec![
            (vk::Format::R32G32_SFLOAT, 0),
            (vk::Format::R32G32_SFLOAT, 0),
            (vk::Format::R8G8B8A8_SNORM, 0)
        ]
    }
}

struct InterfaceCreateInfo<'a> {
    fonts : FontDefinitions,
    style : Style,
    pixel_per_point : f32,

    renderer : &'a Renderer,
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

        let mut winit = egui_winit::State::new(context.clone(),
            ViewportId::ROOT,
            target,
            Some(info.pixel_per_point),
            None);

        // Create a render pass.
        let render_pass = RenderPassCreateInfo::default()
            .color_attachment(
                info.renderer.swapchain.surface_format.format,
                vk::SampleCountFlags::TYPE_1,
                vk::AttachmentLoadOp::DONT_CARE,
                vk::AttachmentStoreOp::STORE,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
            )
            .dependency(
                vk::SUBPASS_EXTERNAL,
                0,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE
            )
            .subpass(vk::PipelineBindPoint::GRAPHICS, &[0], &[], None)
            .build(&info.renderer.device);

        // Create a descriptor pool.
        let descriptor_set_layout = DescriptorSetLayoutBuilder::default()
            .sets(1024)
            .binding(0, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT, PoolDescriptorCount(1024), BindingDescriptorCount(1))
            .build(&info.renderer);

        let pipeline_layout = PipelineLayoutInfo::default()
            .layout(&descriptor_set_layout)
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
            .add_shader("./assets/gui.vert".into(), vk::ShaderStageFlags::VERTEX)
            .add_shader("./assets/gui.frag".into(), vk::ShaderStageFlags::FRAGMENT)
            .build(&info.renderer.device);

        todo!()
    }

    pub fn begin_frame(&mut self, window : &Window) {
        let raw_input = self.egui.take_egui_input(window.handle());
        self.context.begin_frame(raw_input);
    }

    pub fn end_frame(&mut self, window : &Window) -> egui::FullOutput {
        let output = self.context.end_frame();
        self.egui.handle_platform_output(window.handle(), output.platform_output);

        // output
        todo!()
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
}