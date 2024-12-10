mod engine_gui;
mod project_manager;
use eframe::*;
mod audio_engine;
mod ecs;
mod physics_engine;
mod render_engine;

fn main() {
    // Set the native options for the window
    let native_options: NativeOptions = NativeOptions {
        vsync: true, // Enable vertical sync
        hardware_acceleration: HardwareAcceleration::Preferred, // Use default hardware acceleration
        ..Default::default() // Use default values for other fields
    };

    // Run the app with the window title and options
    let _ = eframe::run_native(
        "Rust 2D Game Engine",
        native_options,
        Box::new(|_cc| Ok(Box::new(engine_gui::EngineGui::default()))),
    );
}