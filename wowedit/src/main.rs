use renderer::application::{Application, ApplicationOptions, ApplicationRenderError};
use renderer::vk::descriptor::layout::DescriptorSetLayoutBuilder;
use renderer::vk::pipeline::layout::PipelineLayoutInfo;
use renderer::vk::renderer::{DynamicState, RendererOptions};

use ash::vk;
use winit::event::WindowEvent;

mod casc;
mod rendering;

pub struct ApplicationData {

}

struct TerrainVertex {
    height : f32,
}

fn setup(app : &mut Application) -> ApplicationData {
    let descriptor_set_layout = DescriptorSetLayoutBuilder::default()
        .binding(0, vk::DescriptorType::UNIFORM_BUFFER, vk::ShaderStageFlags::ALL, 1)
        .binding(1, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::ALL, 1)
        .build(&app.renderer.logical_device);

    let pipeline_layout = PipelineLayoutInfo::default()
        .layout(&descriptor_set_layout);

    ApplicationData {
        
    }
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
