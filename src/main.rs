mod engine_gui;
mod gui;
mod project_manager;
use eframe::*;
mod audio_engine;
mod ecs;
mod input_handler;
mod physics_engine;
mod render_engine;
mod game_runtime;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([800.0, 600.0])
            .with_maximized(true),
        ..Default::default()
    };

    // Run the app
    eframe::run_native(
        "Rust Game Engine",
        options,
        Box::new(|cc| Ok(Box::new(engine_gui::EngineGui::new(cc)))),
    )
}
