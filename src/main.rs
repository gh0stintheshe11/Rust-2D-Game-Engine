mod engine_gui;
mod project_file_manager;
use eframe::*;
mod physics_engine;

fn main() {
    // Set the native options for the window
    let native_options: NativeOptions = NativeOptions {
        vsync: true, // Enable vertical sync
        hardware_acceleration: HardwareAcceleration::Preferred, // Use default hardware acceleration
        run_and_return: false, // Don't run the app and return the window
        centered: true, // Center the window
        ..Default::default() // Use default values for other fields
    };

    // Run the app with the window title and options
    let _ = eframe::run_native(
        "Rust 2D Game Engine",
        native_options,
        Box::new(|_cc| Ok(Box::new(engine_gui::EngineGui::default()))),
    );
}