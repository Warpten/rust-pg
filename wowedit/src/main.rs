use std::mem::{offset_of, size_of};

use renderer::application::{Application, ApplicationOptions, ApplicationRenderError};
use renderer::traits::handle::Handle;
use renderer::vk::buffer::{Buffer, BufferBuilder};
use renderer::vk::descriptor::layout::DescriptorSetLayoutBuilder;
use renderer::vk::pipeline::layout::PipelineLayoutInfo;
use renderer::vk::pipeline::{DepthOptions, Pipeline, PipelineInfo, Vertex};
use renderer::vk::renderer::{DynamicState, RendererOptions};

use ash::vk;
use winit::event::WindowEvent;

mod casc;
mod rendering;

pub struct ApplicationData {
    pipeline : Pipeline,
    buffer : Buffer,
}

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

    fn format_offset() -> Vec<(vk::Format, u32)> {
        vec![
            (vk::Format::R32G32_SFLOAT,    offset_of!(TerrainVertex, pos) as u32),
            (vk::Format::R32G32B32_SFLOAT, offset_of!(TerrainVertex, color) as u32)
        ]
    }
}

fn setup(app : &mut Application) -> ApplicationData {
    let buffer = BufferBuilder::<TerrainVertex>::default()
        .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
        .gpu_only()
        .data(&[
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
        ])
        .build(&app.renderer);

    let descriptor_set_layout = DescriptorSetLayoutBuilder::default()
        // .binding(0, vk::DescriptorType::UNIFORM_BUFFER, vk::ShaderStageFlags::ALL, 1)
        // .binding(1, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::ALL, 1)
        .build(&app.renderer);

    let pipeline_layout = PipelineLayoutInfo::default()
        .layout(&descriptor_set_layout)
        .build(&app.renderer);

    let pipeline = PipelineInfo::default()
        .layout(pipeline_layout.handle())
        .depth(DepthOptions::enabled())
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::CLOCKWISE)
        .render_pass(app.renderer.render_pass.handle())
        .samples(app.renderer.options().multisampling)
        .pool(&app.renderer.pipeline_cache)
        .vertex::<TerrainVertex>()
        .add_shader("./assets/triangle.vert".into(), vk::ShaderStageFlags::VERTEX)
        .add_shader("./assets/triangle.frag".into(), vk::ShaderStageFlags::FRAGMENT)
        .build(&app.renderer.device);

    ApplicationData {
        pipeline,
        buffer,
    }
}

fn prepare() -> ApplicationOptions {
    ApplicationOptions::default()
        .title("Send help")
        .renderer(RendererOptions::default()
            .line_width(DynamicState::Fixed(1.0f32))
            .resolution([1280, 720])
            .multisampling(vk::SampleCountFlags::TYPE_4)
        )
}

pub fn render(app: &mut Application, data: &mut ApplicationData) -> Result<(), ApplicationRenderError> {
    let viewport = vk::Viewport::default()
        .x(0.0f32)
        .y(0.0f32)
        .min_depth(0.0f32)
        .max_depth(1.0f32)
        .width(app.renderer.swapchain.extent.width as _)
        .height(app.renderer.swapchain.extent.height as _);

    let scissors = vk::Rect2D::default()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(app.renderer.swapchain.extent);

    let (image_acquired, cmd) = app.renderer.begin_frame()?;

    cmd.bind_pipeline(vk::PipelineBindPoint::GRAPHICS, &data.pipeline);
    cmd.set_viewport(0, &[viewport]);
    cmd.set_scissors(0, &[scissors]);
    cmd.bind_vertex_buffers(0, &[&data.buffer], &[0]);
    cmd.draw(data.buffer.element_count(), 1, 0, 0);

    app.renderer.end_frame(image_acquired, cmd)
}

pub fn window_event(app: &mut Application, data: &mut ApplicationData, event: &WindowEvent) {
    // Handle keyboard events, etc
}

fn main() {
    Application::build(setup)
        .prepare(prepare)
        .render(render)
        .window_event(window_event)
        .run();
}
