#[allow(dead_code)]

use egui::Context;
use interface::InterfaceState;
use renderer::application::{Application, ApplicationOptions, RendererError};
use renderer::gui::context::{Interface, InterfaceOptions};
use renderer::orchestration::rendering::Orchestrator;
use renderer::vk::renderer::{DynamicState, RendererOptions};

use ash::vk;
use rendering::geometry::GeometryRenderer;
use theming::aesthetix::Aesthetix;
use winit::event::WindowEvent;

mod events;
mod interface;
mod theming;
mod rendering;

pub struct ApplicationData { // Get rid of this
}

fn setup(app : &mut Application) -> ApplicationData {
    ApplicationData { }
}

fn prepare() -> ApplicationOptions {
    ApplicationOptions::default()
        .title("Send help")
        .device_extension(ash::khr::swapchain::NAME.to_owned())
        .renderer(RendererOptions::default()
            .line_width(DynamicState::Fixed(1.0f32))
            .resolution([1280, 720])
            .multisampling(vk::SampleCountFlags::TYPE_4)
        )
        .orchestrator(|context| {
            Orchestrator::new(context)
                .add_renderer(|ctx, swapchain| Box::new(GeometryRenderer::supplier(swapchain, ctx, false)))
                .add_renderer(|ctx, swapchain| {
                    let theme = theming::themes::StandardDark{};
                    let style = theme.custom_style();

                    let options = InterfaceOptions {
                        style,
                        ..Default::default()
                    };

                    Box::new(Interface::supplier(swapchain, ctx, true, render_interface, options))
                })
        })
}

pub fn render(app: &mut Application, data: &mut ApplicationData) -> Result<(), RendererError> {
    app.orchestrator.draw_frame()
}

pub fn window_event(app: &mut Application, data : &mut ApplicationData, event: &WindowEvent) {
    _ = app.orchestrator.handle_event(&event);
}

fn main() {
    Application::build(setup)
        .prepare(prepare)
        .render(render)
        .window_event(window_event)
        .run();
}

#[inline] fn render_interface(ctx : &Context, state : &mut InterfaceState) {
    state.render(ctx);
}
