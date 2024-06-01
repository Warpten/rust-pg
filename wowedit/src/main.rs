use std::mem::{offset_of, size_of};

use egui::{FontDefinitions, Style};
use renderer::application::{Application, ApplicationOptions, ApplicationRenderError};
use renderer::gui::context::{Interface, InterfaceCreateInfo};
use renderer::traits::handle::Handle;
use renderer::vk::buffer::{Buffer, DynamicBufferBuilder, DynamicInitializer};
use renderer::vk::descriptor::layout::{DescriptorSetLayout, DescriptorSetLayoutBuilder};
use renderer::vk::framebuffer::Framebuffer;
use renderer::vk::pipeline::layout::PipelineLayoutInfo;
use renderer::vk::pipeline::{DepthOptions, Pipeline, PipelineInfo, Vertex};
use renderer::vk::render_pass::{RenderPass, SubpassAttachment};
use renderer::vk::renderer::{DynamicState, RendererOptions};

use ash::vk;
use rendering::geometry::GeometryRendererBuilder;
use winit::event::WindowEvent;

mod casc;
mod rendering;

pub struct ApplicationData {
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

fn setup(app : &mut Application) -> ApplicationData {
    ApplicationData { }
}

fn prepare() -> ApplicationOptions {
    ApplicationOptions::default()
        .title("Send help")
        .renderer(RendererOptions::default()
            .line_width(DynamicState::Fixed(1.0f32))
            .resolution([1280, 720])
            .multisampling(vk::SampleCountFlags::TYPE_4)
        )
        .add_renderable(|_device, _swapchain, _pipeline_cache| {
            Box::new(GeometryRendererBuilder { })
        })
        /*.add_renderable(|device, swapchain, pipeline_cache| {
            Box::new(Interface::builder())
        })*/
}

pub fn render(app: &mut Application, data: &mut ApplicationData) -> Result<(), ApplicationRenderError> {
    app.renderer.draw_frame()
}

pub fn window_event(app: &mut Application, data: &mut ApplicationData, event: &WindowEvent) {
    // _ = app.renderer.gui.handle_event(&event, &app.window);
}

fn main() {
    Application::build(setup)
        .prepare(prepare)
        .render(render)
        .window_event(window_event)
        .run();
}
