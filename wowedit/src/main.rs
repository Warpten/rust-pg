use renderer::application::{Application, ApplicationOptions, RendererError};
use renderer::gui::context::Interface;
use renderer::orchestration::rendering::Orchestrator;
use renderer::vk::renderer::{DynamicState, RendererOptions};

use ash::vk;
use rendering::geometry::GeometryRenderer;
use winit::event::WindowEvent;

mod casc;
mod rendering;

pub struct ApplicationData { // Get rid of this
}

fn setup(app : &mut Application) -> ApplicationData {
    ApplicationData { }
}

fn render_interface(context : &Interface) {
    egui::TopBottomPanel::top("top_bar").show(&context.context, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.visuals_mut().button_frame = false;
            egui::widgets::global_dark_light_mode_switch(ui);
            ui.visuals_mut().button_frame = true;

            ui.separator();

            
        });
    });
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
                .add_renderer(|ctx| Box::new(GeometryRenderer::supplier(ctx, false)))
                .add_renderer(|ctx| Box::new(Interface::supplier(ctx, true, render_interface)))
        })
}

pub fn render(app: &mut Application, data: &mut ApplicationData) -> Result<(), RendererError> {
    app.orchestrator.draw_frame()
}

pub fn window_event(app: &mut Application, data: &mut ApplicationData, event: &WindowEvent) {
    _ = app.orchestrator.handle_event(&event);
}

fn main() {
    Application::build(setup)
        .prepare(prepare)
        .render(render)
        .window_event(window_event)
        .run();
}
