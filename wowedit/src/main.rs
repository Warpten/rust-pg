use std::mem::{offset_of, size_of};

use renderer::application::{Application, ApplicationOptions, RendererError};
use renderer::gui::context::Interface;
use renderer::vk::pipeline::Vertex;
use renderer::vk::renderer::{DynamicState, RendererOptions};

use ash::vk;
use rendering::geometry::GeometryRendererBuilder;
use winit::event::WindowEvent;

mod casc;
mod rendering;

pub struct ApplicationData {
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
        .add_renderable(|_device, _swapchain, _pipeline_cache| {
            Box::new(Interface::builder())
        })
}

pub fn render(app: &mut Application, data: &mut ApplicationData) -> Result<(), RendererError> {
    app.renderer.draw_frame()
}

pub fn window_event(app: &mut Application, data: &mut ApplicationData, event: &WindowEvent) {
    _ = app.renderer.handle_event(&event);
}

fn main() {
    Application::build(setup)
        .prepare(prepare)
        .render(render)
        .window_event(window_event)
        .run();
}
