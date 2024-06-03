use renderer::application::{Application, ApplicationOptions, RendererError};
use renderer::gui::context::Interface;
use renderer::orchestration::rendering::Orchestrator;
use renderer::vk::renderer::{DynamicState, RendererOptions};

use ash::vk;
use rendering::geometry::GeometryRenderer;
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
        .orchestrator(|context| {
            Orchestrator::new(context)
                .add_renderer(|c, i| Box::new(GeometryRenderer::supplier(c, i)))
                // .add_renderer(|c, i| Box::new(Interface::supplier(c, i)))
        })
}

pub fn render(app: &mut Application, data: &mut ApplicationData) -> Result<(), RendererError> {
    app.orchestrator.draw_frame()
}

pub fn window_event(app: &mut Application, data: &mut ApplicationData, event: &WindowEvent) {
    // _ = app.orchestrator.handle_event(&event);
}

fn main() {
    Application::build(setup)
        .prepare(prepare)
        .render(render)
        .window_event(window_event)
        .run();
}
