use std::time::{Duration, Instant, SystemTime};

use egui::{Context, RichText, Ui};

#[derive(Default)]
pub struct InterfaceState {
    // Toggles display of GUI memory usage
    pub gui_memory_profiler : bool,
    // Toggles Puffer GUI (CPU profiler)
    pub cpu_profiler : bool,

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

impl InterfaceState {
    pub fn render(&mut self, ctx : &Context) {
        egui::Window::new("GUI statistics")
            .open(&mut self.gui_memory_profiler)
            .resizable(true)
            .show(ctx, |ui| {
                puffin::set_scopes_on(true);
                puffin_egui::profiler_ui(ui);
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
                        ui.heading(egui::RichText::new("World Edit").size(25.0).strong());
                    });

                ui.with_layout(
                    egui::Layout::top_down_justified(egui::Align::Min),
                    |ui| {
                        ui.selectable_value(&mut self.active_tab, Tab::Home, "🏠 Home");
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
                    Tab::Home => self.render_home(ctx, ui),
                    Tab::Database => self.render_database(ctx, ui),
                    Tab::World => self.render_world(ctx, ui),
                    Tab::Model => self.render_model(ctx, ui),
                    Tab::Explorer => self.render_explorer(ctx, ui),
                    Tab::Settings => self.render_settings(ctx, ui),
                    Tab::About => self.render_about(ctx, ui),
                }
            });
    }

    fn render_home(&mut self, ctx : &Context, ui : &mut Ui) {

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
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                    egui::CollapsingHeader::new("Open-source licenses").show(ui, |ui| {
                        ui.label("I'll add these soon enough.");
                    });
                });
            });
        });
    }
}