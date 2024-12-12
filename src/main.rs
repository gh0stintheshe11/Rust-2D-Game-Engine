mod engine_gui;
mod project_manager;
use eframe::*;
mod audio_engine;
mod ecs;
mod input_handler;
mod physics_engine;
mod render_engine;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        ..Default::default()
    };

    // Create the app
    eframe::run_native(
        "Rust 2D Game Engine",
        options,
        Box::new(|cc| {
            // Create the app
            let app = Box::new(engine_gui::EngineGui::default());

            // Set up event handling
            cc.egui_ctx.set_visuals(egui::Visuals::dark());

            Ok(app)
        }),
    )
}
