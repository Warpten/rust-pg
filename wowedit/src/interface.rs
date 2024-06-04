use egui::Context;

#[derive(Default)]
pub struct InterfaceState {
    // Toggles display of GUI memory usage
    pub gui_memory_profiler : bool,
    // Toggles Puffer GUI (CPU profiler)
    pub cpu_profiler : bool,
}

impl InterfaceState {
    pub fn render(&mut self, ctx : &Context) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.visuals_mut().button_frame = false;
                egui::widgets::global_dark_light_mode_switch(ui);
                ui.visuals_mut().button_frame = true;
    
                ui.separator();
    
                ui.checkbox(&mut self.gui_memory_profiler, "Show GUI statistics");
                ui.checkbox(&mut self.cpu_profiler, "Show CPU profiler");
    
                ui.separator();
            });
    
            egui::Window::new("GUI statistics")
                .open(&mut self.gui_memory_profiler)
                .resizable(true)
                .show(ctx, |ui| {
                    puffin::set_scopes_on(true);
                    puffin_egui::profiler_ui(ui);
                });
        });
    }
}