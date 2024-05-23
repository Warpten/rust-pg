use renderer::vk::{renderer::{DynamicState, RendererOptions}};
use winit::event::WindowEvent;
use renderer::application::{Application, ApplicationOptions, ApplicationRenderError};

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
        )
}

pub fn render(app: &mut Application, data: &mut ApplicationData) -> Result<(), ApplicationRenderError> {
    let renderer = app.renderer();
    let (semaphore, frame_index) = renderer.acquire_next_image()?;

    // 1. Acquire a command buffer.
    // 2. Begin the render pass.
    // 3. 

    renderer.submit_and_present(ash::vk::CommandBuffer::null(), semaphore);
    Ok(()) 
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
