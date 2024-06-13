use std::path::Path;

#[allow(dead_code)]

use egui::Context;
use egui::{FontData, FontDefinitions, FontFamily};
use interface::InterfaceState;
use renderer::application::{Application, ApplicationOptions, RendererError};
use renderer::gui::context::{Interface, InterfaceOptions};
use renderer::orchestration::orchestrator::Orchestrator;
use renderer::vk::renderer::{DynamicState, RendererOptions};

use ash::vk;
use rendering::geometry::GeometryRenderer;
use winit::event::WindowEvent;

mod events;
mod interface;
mod theming;
mod rendering;

pub struct ApplicationData {
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
                .add_renderer(|ctx, swapchain| Box::new(GeometryRenderer::supplier(swapchain, ctx, false)), None, None)
                .add_renderer(|ctx, swapchain| {
                    let _theme = theming::themes::StandardDark{};
                    let style = egui::Style::default(); // _theme.custom_style();

                    let mut fonts = FontDefinitions::default();
                    load_fonts(&mut fonts, &None, "./assets/fonts");
                    for (k, v) in &fonts.families { println!("Loaded {:?} {:?}", k, v); }

                    let options = InterfaceOptions::default(render_interface)
                        .fonts(fonts)
                        .style(style);

                    Box::new(Interface::new(swapchain, ctx, true, options))
                }, None, None)
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