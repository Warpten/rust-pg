use renderer::{Application, ApplicationBuilder, ApplicationOptions, SwapchainOptions, Window};

mod application;
mod casc;
mod exit_signal;
mod vulkan;

struct SwapchainOpts;

struct ApplicationData;

fn setup(app : &mut Application) -> ApplicationData {
    ApplicationData { }
}

fn prepare() -> ApplicationOptions {
    ApplicationOptions::default()
        .title("Send help")
        .line_width(1.0f32)
        .resolution([800; 600])
}

pub fn render(app: &mut Application, data: &mut ApplicationData) -> Result<(), ApplicationRenderError> {
    // Issue render calls here?
    Some(()) 
}

pub fn window_event(_: &mut Application, data: &mut ApplicationData, event: &winit::event::WindowEvent) {
    // Handle keyboard events, etc
}

fn main() {
    Application::build(setup)
        .prepare(prepare)
        .render(render)
        .window_event(window_event)
        .run();
}
