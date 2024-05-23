use renderer::vk::{renderer::{DynamicState, RendererOptions}, SwapchainOptions};
use winit::event::WindowEvent;
use renderer::application::{Application, ApplicationOptions, ApplicationRenderError};

use ash::vk;

mod casc;

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
            .multisampling(vk::SampleCountFlags::TYPE_1)
        )
}

pub fn render(app: &mut Application, data: &mut ApplicationData) -> Result<(), ApplicationRenderError> {
    app.renderer.run_frame(|_device, _cmd| {
        // Do stuff here.
    })
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
