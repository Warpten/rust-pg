use std::mem::{offset_of, size_of};

use ash::vk;
use puffin::profile_scope;
use renderer::{orchestration::{render::Renderer, rendering::{Renderable, RenderingContext}}, traits::handle::Handle, vk::{buffer::{Buffer, DynamicBufferBuilder, DynamicInitializer}, command_pool::CommandPool, frame_data::FrameData, framebuffer::Framebuffer, pipeline::{layout::{PipelineLayout, PipelineLayoutInfo}, DepthOptions, Pipeline, PipelineInfo, Vertex}, render_pass::{RenderPass, SubpassAttachment}, swapchain::Swapchain}};

#[derive(Copy, Clone)]
struct TerrainVertex {
    pos : [f32; 2],
    color : [f32; 3],
}

impl Vertex for TerrainVertex {
    fn bindings() -> Vec<(u32, vk::VertexInputRate)> {
        vec![
            (size_of::<Self>() as u32, vk::VertexInputRate::VERTEX)
        ]
    }

    fn format_offset() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription::default()
                .format(vk::Format::R32G32_SFLOAT)
                .binding(0)
                .location(0)
                .offset(offset_of!(TerrainVertex, pos) as u32),
            vk::VertexInputAttributeDescription::default()
                .format(vk::Format::R32G32B32_SFLOAT)
                .binding(0)
                .location(1)
                .offset(offset_of!(TerrainVertex, color) as u32),
        ]
    }
}

impl Renderable for GeometryRenderer {    
    fn create_framebuffers(&mut self, swapchain : &Swapchain) {
        self.framebuffers.clear();
        for image in &swapchain.images {
            self.framebuffers.push(self.render_pass.create_framebuffer(swapchain, image));
        }
    }

    fn record_commands(&mut self, swapchain : &Swapchain, frame : &FrameData) {
        profile_scope!("Geometry command recording");

        let viewport = vk::Viewport::default()
            .x(0.0f32)
            .y(0.0f32)
            .min_depth(0.0f32)
            .max_depth(1.0f32)
            .width(swapchain.extent.width as _)
            .height(swapchain.extent.height as _);

        let scissors = vk::Rect2D::default()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(swapchain.extent);

        frame.cmd.begin_render_pass(&self.render_pass, &self.framebuffers[frame.index], vk::Rect2D {
            offset : vk::Offset2D { x: 0, y : 0 },
            extent : swapchain.extent
        }, &[
            vk::ClearValue {
                color : vk::ClearColorValue {
                    float32: [0.0; 4],
                },
            },
            vk::ClearValue {
                depth_stencil : vk::ClearDepthStencilValue {
                    depth : 1.0f32,
                    stencil : 0,
                }
            }
        ], vk::SubpassContents::INLINE);
        frame.cmd.bind_pipeline(vk::PipelineBindPoint::GRAPHICS, &self.pipeline);
        frame.cmd.set_viewport(0, &[viewport]);
        frame.cmd.set_scissors(0, &[scissors]);
        frame.cmd.bind_vertex_buffers(0, &[(&self.buffer, 0)]);
        frame.cmd.draw(self.buffer.element_count(), 1, 0, 0);
        frame.cmd.end_render_pass();
    }

    fn marker_data<'a>(&self) -> (&'a str, [f32; 4]) {
        ("Geometry renderer", [0.0; 4])
    }
}

pub struct GeometryRenderer {
    buffer : Buffer,
    transfer_pool : CommandPool,
    framebuffers : Vec<Framebuffer>,
    // descriptor_set_layout : DescriptorSetLayout,
    pipeline_layout : PipelineLayout,
    pipeline : Pipeline,
    render_pass : RenderPass,
}

impl GeometryRenderer {
    pub fn new(renderer : &Renderer, is_presenting : bool) -> Self {
        let render_pass = renderer.swapchain.create_render_pass(is_presenting)
            .dependency(
                vk::SUBPASS_EXTERNAL,
                0,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::AccessFlags::empty(),
                vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE
            ).subpass(vk::PipelineBindPoint::GRAPHICS, &[
                SubpassAttachment::color(0),
                SubpassAttachment::resolve(0)
            ], None)
            .build(&renderer.context);

        let transfer_pool = CommandPool::builder(&renderer.context.transfer_queue)
            .build(&renderer.context);

        let buffer = DynamicBufferBuilder::dynamic()
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .gpu_only()
            .build(&renderer.context, &transfer_pool, &[
                TerrainVertex {
                    pos : [ 0.0f32, -0.5f32 ],
                    color : [ 1.0f32, 0.0f32, 0.0f32 ]
                },
                TerrainVertex {
                    pos : [ 0.5f32, 0.5f32 ],
                    color : [ 0.0f32, 1.0f32, 0.0f32 ]
                },
                TerrainVertex {
                    pos : [ -0.5f32, 0.5f32 ],
                    color : [ 0.0f32, 0.0f32, 1.0f32 ]
                }
            ]);

        // let descriptor_set_layout = DescriptorSetLayout::builder()
        //     .build(&context.device);

        let pipeline_layout = PipelineLayoutInfo::default()
        //     .layout(&descriptor_set_layout)
            .build(&renderer.context);

        let pipeline = PipelineInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .layout(pipeline_layout.handle())
            .depth(DepthOptions::enabled())
            .color_blend_attachment(vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(false)
                .src_color_blend_factor(vk::BlendFactor::SRC_COLOR)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_DST_COLOR)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ZERO)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::RGBA))
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .render_pass(render_pass.handle(), 0)
            .samples(renderer.context.options.multisampling)
            .pool()
            .vertex::<TerrainVertex>()
            .add_shader("./assets/triangle.vert".into(), vk::ShaderStageFlags::VERTEX)
            .add_shader("./assets/triangle.frag".into(), vk::ShaderStageFlags::FRAGMENT)
            .build(&renderer.context);

        Self {
            buffer,
            transfer_pool,
            // descriptor_set_layout,
            pipeline_layout,
            pipeline,
            framebuffers : {
                let mut framebuffers = vec![];
                for image in &renderer.swapchain.images {
                    framebuffers.push(render_pass.create_framebuffer(&renderer.swapchain, image));
                }
                framebuffers
            },
            render_pass,
        }
    }
}
