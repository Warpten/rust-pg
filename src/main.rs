#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use application::{Application, debugging_state::DebuggingState};

mod application;
mod casc;
mod vulkan;

fn main() { }
/*
fn main() -> Result<(), eframe::Error> {
    #[cfg(debug_assertions)]
    let state = {
        let mut state = DebuggingState::default();
        for arg in std::env::args().skip(1) {
            match arg.as_str() {
                "--debug-memory" => {
                    state.memory = true;
                }, "--profiler" => {
                    state.profiler = true;
                }, _ => panic!("Unknown argument: {arg}")
            }
        }

        Some(state)
    };

    #[cfg(not(debug_assertions))]
    let state = Option::None;

    eframe::run_native("WorldEdit",
        eframe::NativeOptions {
            viewport : egui::ViewportBuilder::default()
                .with_inner_size([1280.0, 1024.0])
                .with_drag_and_drop(false),

            ..Default::default()
        },
        Box::new(|cc| Box::new(Application::new(cc, state))),
    )
}
*/