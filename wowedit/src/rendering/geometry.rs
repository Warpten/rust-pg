use std::{mem::{offset_of, size_of}, sync::Arc};

use ash::vk;
use egui_winit::EventResponse;
use renderer::{orchestration::{renderer::RenderingContext, traits::{Renderable, RenderableFactory}}, traits::handle::Handle, vk::{buffer::{Buffer, DynamicBufferBuilder, DynamicInitializer}, command_buffer::CommandBuffer, command_pool::{CommandPool, CommandPoolBuilder}, descriptor::layout::DescriptorSetLayout, pipeline::{layout::{PipelineLayout, PipelineLayoutInfo}, DepthOptions, Pipeline, PipelineInfo, Vertex}, render_pass::RenderPassCreateInfo, swapchain::Swapchain}};
use renderer::vk::render_pass::SubpassAttachment;

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

pub struct GeometryRendererBuilder {

}

impl RenderableFactory for GeometryRendererBuilder {
    fn build(&self, context : &std::sync::Arc<RenderingContext>) -> Box<dyn Renderable> {
        Box::new(GeometryRenderer::new(self, context))
    }

    fn express_dependencies(&self, create_info : RenderPassCreateInfo) -> RenderPassCreateInfo {
        create_info
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
            ], SubpassAttachment::depth(0).into())
    }
}

pub struct GeometryRenderer {
    buffer : Buffer,
    transfer_pool : CommandPool,
    descriptor_set_layout : DescriptorSetLayout,
    pipeline_layout : PipelineLayout,
    pipeline : Pipeline,
    swapchain : Arc<Swapchain>,
}

impl GeometryRenderer {
    pub fn new(builder : &GeometryRendererBuilder, context : &RenderingContext) -> Self {
        let transfer_pool = CommandPool::builder(&context.transfer_queue)
            .build(&context.device);

        let buffer = DynamicBufferBuilder::dynamic()
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .gpu_only()
            .build(&context.device, &transfer_pool, &[
                TerrainVertex {
                    pos : [ 0.0f32, -0.5f32],
                    color : [ 1.0f32, 0.0f32, 0.0f32 ]
                },
                TerrainVertex {
                    pos : [ 0.5f32, 0.5f32],
                    color : [ 0.0f32, 1.0f32, 0.0f32 ]
                },
                TerrainVertex {
                    pos : [ -0.5f32, 0.5f32],
                    color : [ 0.0f32, 0.0f32, 1.0f32 ]
                }
            ]);

        let descriptor_set_layout = DescriptorSetLayout::builder()
            .build(&context.device);

        let pipeline_layout = PipelineLayoutInfo::default()
            .layout(&descriptor_set_layout)
            .build(&context.device);

        let pipeline = PipelineInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .layout(pipeline_layout.handle())
            .depth(DepthOptions::enabled())
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .render_pass(context.render_pass.handle(), 0)
            .samples(context.options.multisampling)
            .pool(&context.pipeline_cache)
            .vertex::<TerrainVertex>()
            .add_shader("./assets/triangle.vert".into(), vk::ShaderStageFlags::VERTEX)
            .add_shader("./assets/triangle.frag".into(), vk::ShaderStageFlags::FRAGMENT)
            .build(&context.device);

        Self {
            buffer,
            transfer_pool,
            descriptor_set_layout,
            pipeline_layout,
            pipeline,
            swapchain : context.swapchain.clone(),
        }
    }
}

impl Renderable for GeometryRenderer {
    fn draw_frame(&mut self, cmd : &CommandBuffer, _frame_index : usize) {
        let viewport = vk::Viewport::default()
            .x(0.0f32)
            .y(0.0f32)
            .min_depth(0.0f32)
            .max_depth(1.0f32)
            .width(self.swapchain.extent.width as _)
            .height(self.swapchain.extent.height as _);

        let scissors = vk::Rect2D::default()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(self.swapchain.extent);

        cmd.label("Draw application frame".to_owned(), [1.0, 0.0, 0.0, 0.0], || {
            cmd.bind_pipeline(vk::PipelineBindPoint::GRAPHICS, &self.pipeline);
            cmd.set_viewport(0, &[viewport]);
            cmd.set_scissors(0, &[scissors]);
            cmd.bind_vertex_buffers(0, &[(&self.buffer, 0)]);
            cmd.draw(self.buffer.element_count(), 1, 0, 0);
        });
    }

    fn handle_event(&mut self, event : &winit::event::WindowEvent, window : &renderer::window::Window) -> Option<EventResponse> {
        None
    }
    
    fn contents_type(&self) -> vk::SubpassContents { vk::SubpassContents::INLINE }
}