mod engine_gui;
mod project_file_manager;
use eframe::NativeOptions;

fn main() {
    // Set the native options for the window
    let native_options: NativeOptions = NativeOptions {
        vsync: true, // Enable vertical sync
        ..Default::default() // Use default values for other fields
    };

    // Run the app with the window title and options
    eframe::run_native(
        "Rust 2D Game Engine",
        native_options,
        Box::new(|_cc| Ok(Box::new(engine_gui::EngineGui::default()))), // Wrap in Ok
    );
}