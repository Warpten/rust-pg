use std::borrow::BorrowMut;
#[allow(dead_code)]

use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use egui::{FontData, FontDefinitions, FontFamily};
use interface::InterfaceState;
use renderer::application::{Application, ApplicationOptions, RendererError};
use renderer::gui::context::{InterfaceRenderer, InterfaceOptions};
use renderer::orchestration::render::Renderer;
use renderer::vk::renderer::{DynamicState, RendererOptions};

use ash::vk;
use renderer::window::Window;
use rendering::geometry::GeometryRenderer;
use tokio::runtime::{Builder, Runtime};
use winit::event::WindowEvent;
use crate::application::{ApplicationData, SharedState};
use crate::async_task_manager::AsyncTaskManager;

mod application;
mod events;
mod interface;
mod theming;
mod rendering;
mod async_task_manager;

fn make_vk_renderer(app: &mut Application, window : Window) -> Renderer {
    let renderer = RendererOptions::default()
        .line_width(DynamicState::Fixed(1.0f32))
        .multisampling(vk::SampleCountFlags::TYPE_4);

    Renderer::builder(app.context.clone())
        .build(renderer, window, vec![ash::khr::swapchain::NAME.to_owned()])
}

fn make_geometry_renderer(renderer : &mut Renderer, shared_state : Arc<RwLock<SharedState>>) -> GeometryRenderer<SharedState> {
    GeometryRenderer::new(renderer, false, shared_state.clone())
}

fn make_interface_renderer(renderer : &mut Renderer, shared_state : Arc<RwLock<SharedState>>) -> InterfaceRenderer<InterfaceState> {
    let _theme = theming::themes::StandardDark{};
    let style = egui::Style::default(); // _theme.custom_style();

    let mut fonts = FontDefinitions::default();
    load_fonts(&mut fonts, &None, "./assets/fonts");

    let state_copy = shared_state.clone();
    let options = InterfaceOptions::default(InterfaceState::default(state_copy))
        .fonts(fonts)
        .style(style);

    InterfaceRenderer::new(&renderer.swapchain, &renderer.context, true, options)
}

fn setup(app : &mut Application, window : Window) -> ApplicationData {
    // State shared across multiple objects
    let shared_state = Arc::new(RwLock::new(SharedState::default()));

    // Shared Vulkan rendering "engine"
    let mut renderer = make_vk_renderer(app, window);

    // Individual renderers
    let geometry  = make_geometry_renderer(&mut renderer, shared_state.clone());
    let interface = make_interface_renderer(&mut renderer, shared_state.clone());

    // Return the application data now
    ApplicationData {
        geometry,
        interface,
        renderer,

        shared_state,
    }
}

fn prepare() -> ApplicationOptions {
    ApplicationOptions::default()
        .title("Send help")
        .resolution([1280, 720])
}

fn render(_app: &mut Application, data: &mut ApplicationData) -> Result<(), RendererError> {
    { // Scoped lock
        let mut state = data.shared_state.write().unwrap();
        state.fs.try_poll();
    }

    data.updater().draw()
}

fn window_event(_app: &mut Application, data : &mut ApplicationData, event: &WindowEvent) {
    _ = data.interface.egui.on_window_event(data.renderer.context.window.handle(), event)
}

fn main() {
    let runtime = Builder::new_multi_thread().enable_all().build().unwrap();

    Application::build(setup)
        .prepare(prepare)
        .render(render)
        .window_event(window_event)
        .run(&runtime);
}

fn load_fonts<P>(def : &mut FontDefinitions, mut family : &Option<FontFamily>, dir : P) where P : AsRef<Path> {
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for sub_path in entries {
            if let Ok(sub_path) = sub_path {
                let file_type = sub_path.file_type();
                if let Ok(file_type) = file_type {
                    let absolute_path = AsRef::<Path>::as_ref(&dir).join(sub_path.file_name());
                    if file_type.is_file() {
                        let file_data = std::fs::read(&absolute_path);
                        if file_data.is_err() {
                            println!("An error occured while loading '{:?}': {}", absolute_path, file_data.err().unwrap());
                            continue;
                        }

                        let font_name = absolute_path.file_stem()
                            .unwrap()
                            .to_str()
                            .unwrap();

                        let index = font_name.find('-').map(|i| i + 1).unwrap_or_default();
                        let font_name = &font_name[index..];

                        if let Some(family) = &family {
                            match family {
                                FontFamily::Name(_) => unreachable!(),
                                value => {
                                    def.font_data.insert(
                                        font_name.to_owned(),
                                        FontData::from_owned(file_data.unwrap())
                                    );

                                    def.families.entry(value.clone())
                                        .and_modify(move |value| value.push(font_name.to_owned()))
                                        .or_insert(vec![font_name.to_owned()]);

                                    def.families.insert(FontFamily::Name(font_name.into()), vec![font_name.to_owned()]);

                                    println!("Loaded {:?} as {}", font_name, value);
                                },
                            }
                        } else {
                            println!("Tried to load font {:?} but this font should be in a subdirectory named 'proportional' or 'monospace'", font_name);
                        }
                    } else {
                        if sub_path.file_name() == "proportional" {
                            family = &Some(FontFamily::Proportional);
                        } else if sub_path.file_name() == "monospace" {
                            family = &Some(FontFamily::Monospace);
                        }

                        load_fonts(def, family, absolute_path);    
                    }

                }
            }
        }
    }
}
