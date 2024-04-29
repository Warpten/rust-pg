use super::debugging_state::DebuggingState;

use egui::{Context, Ui, Window};

#[derive(Debug)]
pub struct WindowState {
    debugging_state : Option<DebuggingState>,
}

impl WindowState {
    pub fn new(state : Option<DebuggingState>) -> WindowState {
        WindowState {
            debugging_state : state
        }
    }

    pub fn checkboxes(&mut self, ui : &mut Ui) {
        match self.debugging_state {
            Some(ref mut state) => {
                ui.checkbox(&mut state.profiler, "Show CPU profiler");
                ui.checkbox(&mut state.memory, "Show GUI memory");
                ui.separator();
            },
            None => ()
        };
    }

    pub fn display_windows(&mut self, ctx : &Context) {
        match self.debugging_state {
            Some(ref mut state) => {
                Window::new("Memory")
                    .open(&mut state.memory)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ctx.memory_ui(ui);
                    });

                let profiler = state.profiler;

                Window::new("Profiler")
                    .default_size([800.0, 600.0])
                    .open(&mut state.profiler)
                    .resizable(false)
                    .show(ctx, |ui| {
                        puffin::set_scopes_on(profiler);
                        puffin_egui::profiler_ui(ui);
                    });
            },
            None => ()
        };
    }
}