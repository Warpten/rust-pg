use eframe::{App, CreationContext, Frame};
use egui::{Context, Grid, TextEdit, Window};

use self::{debugging_state::DebuggingState, window_state::WindowState};

pub(crate) mod debugging_state;
pub(crate) mod window_state;

pub struct Application {
    windows : WindowState,
    show_open_dialog : bool,
    loaded_directory : String,
    load_content_keys : bool,
    load_encoding_specs : bool,
}

impl Application {
    pub fn new(_cc : &CreationContext<'_>, state : Option<DebuggingState>) -> Self {
        Self {
            windows : WindowState::new(state),
            show_open_dialog : false,
            loaded_directory : String::default(),
            load_content_keys : true,
            load_encoding_specs : false,
        }
    }
}

impl App for Application {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        // Give the area behind the floating windows a different color, because it looks better:
        let color = egui::lerp(
            egui::Rgba::from(visuals.panel_fill)..=egui::Rgba::from(visuals.extreme_bg_color),
            0.5,
        );
        let color = egui::Color32::from(color);
        color.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx : &Context, _frame : &mut Frame) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.visuals_mut().button_frame = false;
                egui::widgets::global_dark_light_mode_switch(ui);
                ui.visuals_mut().button_frame = true;

                ui.separator();
                
                self.windows.checkboxes(ui);

                if ui.button("Open...").clicked() {
                    self.show_open_dialog = true
                }
            });

            self.windows.display_windows(ctx);

            Window::new("Open installation")
                .open(&mut self.show_open_dialog)
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    Grid::new("open_grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Installation directory");

                            // TODO: Custom widget, input + button
                            ui.add(TextEdit::singleline(&mut self.loaded_directory).hint_text("Installation path"));
                            ui.end_row();
                        });

                    ui.collapsing("Advanced options", |ui| {
                        ui.vertical(|ui| {
                            ui.checkbox(&mut self.load_content_keys, "Load content keys")
                                .on_hover_ui(|ui| {
                                    ui.label("foo");
                                });
                            ui.checkbox(&mut self.load_encoding_specs, "Load encoding specs");
                        });
                    });

                    ui.button("Load");
                });

        });
    }
}
