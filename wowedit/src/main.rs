use std::path::Path;

#[allow(dead_code)]

use egui::{FontData, FontDefinitions, FontFamily};
use interface::InterfaceState;
use renderer::application::{Application, ApplicationOptions, RendererError};
use renderer::gui::context::{InterfaceRenderer, InterfaceOptions};
use renderer::orchestration::render::{Renderer, RendererAPI, RendererUpdater};
use renderer::vk::renderer::{DynamicState, RendererOptions};

use ash::vk;
use renderer::window::Window;
use rendering::geometry::GeometryRenderer;
use winit::event::WindowEvent;

mod events;
mod interface;
mod theming;
mod rendering;

pub struct ApplicationData {
    renderer : Renderer,
    geometry : GeometryRenderer,
    interface : InterfaceRenderer<InterfaceState>,
}
impl ApplicationData {
    pub fn updater(&mut self) -> RendererUpdater {
        self.renderer.updater(vec![
            &mut self.geometry,
            &mut self.interface
        ])
    }
}
impl RendererAPI for ApplicationData {
    fn is_minimized(&self) -> bool { self.renderer.context.window.is_minimized() }

    fn recreate_swapchain(&mut self) {
        self.updater().recreate_swapchain()
    }

    fn wait_idle(&self) { self.renderer.context.device.wait_idle() }
}

fn setup(app : &mut Application, window : Window) -> ApplicationData {
    let renderer = RendererOptions::default()
        .line_width(DynamicState::Fixed(1.0f32))
        .multisampling(vk::SampleCountFlags::TYPE_4);

    let mut renderer = Renderer::builder(app.context.clone())
        .build(renderer, window, vec![ash::khr::swapchain::NAME.to_owned()]);

    ApplicationData {
        geometry : GeometryRenderer::new(&mut renderer, false),
        interface : {
            let _theme = theming::themes::StandardDark{};
            let style = egui::Style::default(); // _theme.custom_style();

            let mut fonts = FontDefinitions::default();
            load_fonts(&mut fonts, &None, "./assets/fonts");
            for (k, v) in &fonts.families { println!("Loaded {:?} {:?}", k, v); }

            let options = InterfaceOptions::default(|ctx, state : &mut InterfaceState| state.render(ctx))
                .fonts(fonts)
                .style(style);

            InterfaceRenderer::new(&renderer.swapchain, &renderer.context, true, options)
        },
        renderer,
    }
}

fn prepare() -> ApplicationOptions {
    ApplicationOptions::default()
        .title("Send help")
        .resolution([1280, 720])
}

pub fn render(app: &mut Application, data: &mut ApplicationData) -> Result<(), RendererError> {
    data.updater().draw()
}

pub fn window_event(app: &mut Application, data : &mut ApplicationData, event: &WindowEvent) {
    _ = data.interface.egui.on_window_event(data.renderer.context.window.handle(), event)
}

fn main() {
    Application::build(setup)
        .prepare(prepare)
        .render(render)
        .window_event(window_event)
        .run();
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