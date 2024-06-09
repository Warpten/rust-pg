use std::path::{Path, PathBuf};

use egui::{text::LayoutJob, Color32, Context, FontFamily, FontId, FontSelection, Label, Margin, RichText, Style, TextEdit, Ui, Widget};
use egui_extras::{Column, TableBuilder};
use tactfs::psv::PSV;

#[derive(Default)]
pub struct InterfaceState {    
    pub frame_time_profiler  : bool, // Toggles Puffer GUI (CPU profiler)
    pub allocation_breakdown : bool, // Toggles displaying GPU allocation breakdown

    installation_path : String,
    psv_selection : Option<(String, String, String, String, String, PathBuf)>, // Row selected in .build.info

    active_tab : Tab,
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
enum Tab {
    #[default]
    Home,
    Database,
    World,
    Model,
    Explorer,
    Settings,
    About,
}

macro_rules! include_license {
    ($name:expr, $url:expr, $rem:expr, $license_type:expr, $ui:expr) => {
        egui::collapsing_header::CollapsingState::load_with_default_open($ui.ctx(), $ui.make_persistent_id(concat!($name, "-about")), false)
            .show_header($ui, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    egui::Hyperlink::from_label_and_url(RichText::new(format!(concat!("{} ", $name), egui::special_emojis::GITHUB))
                        .size(18.0), $url)
                        .ui(ui);

                        ui.label($rem);
                });
            }).body(|ui| {
                egui::TextEdit::multiline(&mut include_str!(concat!("../../assets/licenses/", $license_type)))
                    .font(egui::TextStyle::Monospace)
                    .interactive(false)
                    .desired_width(f32::INFINITY)
                    .ui(ui);
            });
    };
    ($name:expr, $url:expr,  $license_type:expr, $ui:expr) => {
        egui::collapsing_header::CollapsingState::load_with_default_open($ui.ctx(), $ui.make_persistent_id(concat!($name, "-about")), false)
            .show_header($ui, |ui| {
                egui::Hyperlink::from_label_and_url(RichText::new(format!(concat!("{} ", $name), egui::special_emojis::GITHUB))
                    .size(18.0), $url)
                    .ui(ui);
            }).body(|ui| {
                TextEdit::multiline(&mut include_str!(concat!("../../assets/licenses/", $license_type)))
                    .font(egui::TextStyle::Monospace)
                    .interactive(false)
                    .desired_width(f32::INFINITY)
                    .ui(ui);
            });
    };
}

impl InterfaceState {
    pub fn render(&mut self, ctx : &Context) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                if ui.button("Profiler").clicked() {
                    self.frame_time_profiler = true;
                }

                if ui.button("Allocation breakdown").clicked() {
                    self.allocation_breakdown = true;
                }
            })
        });

        egui::SidePanel::left("main_side_panel")
            .resizable(false)
            .frame(egui::Frame::none()
                .fill(egui::Color32::from_rgb(48, 48, 48))
                .inner_margin(12.0)
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(48, 48, 48),
                )))
            .exact_width(200.0)
            .show(ctx, |ui| {
                ui.with_layout(
                    egui::Layout::top_down_justified(egui::Align::Center),
                    |ui| {
                        ui.add_space(15.0);
                        ui.heading(egui::RichText::new("World Edit")
                            .family(FontFamily::Name("Roboto-Regular".into()))
                            .size(25.0)
                            .strong());
                    });

                ui.with_layout(
                    egui::Layout::top_down_justified(egui::Align::Min),
                    |ui| {
                        let mut label_job = LayoutJob::default();
                        RichText::new("💾")
                            .family(FontFamily::Name("Roboto-Regular".into()))
                            .append_to(&mut label_job, &Style::default(), FontSelection::FontId(FontId {
                                family : FontFamily::Proportional,
                                size : 12.0
                            }), egui::Align::Min);
                        RichText::new("Home")
                            .family(FontFamily::Name("Roboto-Light".into()))
                            .append_to(&mut label_job, &Style::default(), FontSelection::Default, egui::Align::Min);

                        ui.selectable_value(&mut self.active_tab, Tab::Home, label_job);
                        ui.selectable_value(&mut self.active_tab, Tab::Database, "💾 Database");
                        ui.selectable_value(&mut self.active_tab, Tab::World, "🌍 World");
                        ui.selectable_value(&mut self.active_tab, Tab::Model, "🐢 Models");
                        ui.selectable_value(&mut self.active_tab, Tab::Explorer, "📂 File explorer");
                        ui.selectable_value(&mut self.active_tab, Tab::Settings, "⚙ Settings");
                        ui.selectable_value(&mut self.active_tab, Tab::About, "ℹ About");
                    });

                ui.with_layout(
                    egui::Layout::bottom_up(egui::Align::Max),
                    |ui| {
                        ui.horizontal_wrapped(|ui| {
                            if cfg!(debug_assertions) {
                                ui.label(egui::RichText::new("Debug build")
                                    .small()
                                    .color(ui.visuals().warn_fg_color))
                                    .on_hover_text("This is a debug build. Performance may suffer.");
        
                                ui.separator();
                            }
        
                            let rev_string = format!("{}-{}{}",
                                env!("VERGEN_GIT_BRANCH"),
                                env!("VERGEN_GIT_SHA"),
                                if env!("VERGEN_GIT_DIRTY") == "true" { "+" } else { "" }
                            );

                            ui.label(egui::RichText::new(rev_string)
                                .small())
                                .on_hover_text(format!("Compiled {}", env!("VERGEN_BUILD_TIMESTAMP")));
                        });
                    });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .inner_margin(12.0)
                .fill(egui::Color32::from_rgba_premultiplied(30, 30, 30, 127)))
            .show(ctx, |ui| {
                match self.active_tab {
                    Tab::Home     => self.render_home(ctx, ui),
                    Tab::Database => self.render_database(ctx, ui),
                    Tab::World    => self.render_world(ctx, ui),
                    Tab::Model    => self.render_model(ctx, ui),
                    Tab::Explorer => self.render_explorer(ctx, ui),
                    Tab::Settings => self.render_settings(ctx, ui),
                    Tab::About    => self.render_about(ctx, ui),
                }
            });

        egui::Window::new("Profiler")
            .open(&mut self.frame_time_profiler)
            .resizable(true)
            .show(ctx, |ui| {
                puffin::set_scopes_on(true);
                puffin_egui::profiler_ui(ui);
            });

        egui::Window::new("Allocation breakdown")
            .open(&mut self.allocation_breakdown)
            .resizable(true)
            .show(ctx, |ui| {
                ui.label("For various reasons related to dependencies, this does not work yet");
            });
    }

    fn render_home(&mut self, ctx : &Context, ui : &mut Ui) {
        ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
            ui.label(RichText::new("Game installation")
                .size(18.0));

            ui.label("Select the path to your game installation directory");
            egui::TextEdit::singleline(&mut self.installation_path)
                .margin(Margin::symmetric(6.0, 8.0))
                .ui(ui);

            let build_info = PSV::from_file(&Path::new(&self.installation_path).join(".build.info"));
            match build_info {
                Ok(build_info) => {
                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(false)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::auto()) // Version
                        .column(Column::auto()) // Branch
                        .column(Column::remainder()) // Build Key
                        .column(Column::remainder()) // CDN Key
                        .column(Column::remainder()) // Directory
                        .column(Column::exact(100.0)) // Interaction button
                        .min_scrolled_height(0.0)
                        .header(20.0, |mut header| {
                            header.col(|ui| { ui.strong("Version"); });
                            header.col(|ui| { ui.strong("Branch"); });
                            header.col(|ui| { ui.strong("Build Key"); });
                            header.col(|ui| { ui.strong("CDN Key"); });
                            header.col(|ui| { ui.strong("Directory"); });
                            header.col(|_| { });
                        })
                        .body(|mut body| {
                            build_info.for_each_record(move |record| {
                                // Try to find the directory containing this build.
                                let product = record.read("Product").try_raw().unwrap_or("??");

                                if let Some(path_on_disk) = find_flavor_path(&self.installation_path, product) {
                                    body.row(18.0, |mut row | {
                                        let version = record.read("Version").try_raw().unwrap_or("??");
                                        let branch = record.read("Branch").try_raw().unwrap_or("??");
                                        let build_key = record.read("Build Key").try_raw().unwrap_or("??");
                                        let cdn_key = record.read("CDN Key").try_raw().unwrap_or("??");
    
                                        row.col(|ui| { Label::new(version).selectable(false).ui(ui); });
                                        row.col(|ui| { Label::new(branch).selectable(false).ui(ui); });
                                        row.col(|ui| { Label::new(build_key).selectable(false).ui(ui); });
                                        row.col(|ui| { Label::new(cdn_key).selectable(false).ui(ui); });
                                        row.col(|ui| { Label::new(path_on_disk.to_str().unwrap()).selectable(false).ui(ui); });
                                        row.col(|ui| {
                                            if ui.button("Open").clicked() {
                                                self.psv_selection = Some((version.to_string(),
                                                    branch.to_string(),
                                                    build_key.to_string(),
                                                    cdn_key.to_string(),
                                                    product.to_string(),
                                                    path_on_disk
                                                ));
                                            }
                                        });
                                    });
                                }
                            });
                        })
                        ;
                    
                },
                Err(_) => {
                    ui.label(RichText::new("Could not find .build.info.").color(Color32::from_rgb(200, 0, 0)));
                },
            }
        });
    }

    fn render_database(&mut self, ctx : &Context, ui : &mut Ui) {

    }

    fn render_world(&mut self, ctx : &Context, ui : &mut Ui) {

    }

    fn render_model(&mut self, ctx : &Context, ui : &mut Ui) {

    }

    fn render_explorer(&mut self, ctx : &Context, ui : &mut Ui) {

    }

    fn render_settings(&mut self, ctx : &Context, ui : &mut Ui) {

    }

    fn render_about(&mut self, ctx : &Context, ui : &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                ui.collapsing("Open-source licenses", |ui| {
                    include_license!("egui", "https://github.com/emilk/egui/tree/master", "APACHE", ui);

                    include_license!("egui-winit-ash-integration", "https://github.com/MatchaChoco010/egui-winit-ash-integration",
                        "Code from this project is not directly used here but it was referred to so much during development that I felt including it.",
                        "APACHE", ui);

                    include_license!("Aesthetix", "https://github.com/thebashpotato/egui-aesthetix",
                        "Partially used for theming",
                        "MIT-AESTHETIX", ui);
                });
            });
        });
    }
}

fn find_flavor_path(source : &str, product : &str) -> Option<PathBuf> {
    for path in std::fs::read_dir(&source).unwrap() {
        if let Ok(path) = path {
            let is_directory = path.file_type().map(|f| f.is_dir()).unwrap_or(false);
            if !is_directory {
                continue;
            }

            let absolute_path = PathBuf::from(source).join(path.file_name());

            if let Ok(flavor_file) = PSV::from_file(absolute_path.join(".flavor.info")) {
                if let Some(record) = flavor_file.record(0) {
                    if let Ok(flavor) = record.read("Product Flavor").try_raw() {
                        if flavor == product {
                            return Some(absolute_path);
                        }
                    }
                }
            }
        }
    }

    None
}
