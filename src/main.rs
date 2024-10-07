mod gui;
mod project;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(eframe::egui::vec2(1280.0, 720.0)), // Set the initial window size
        ..Default::default()
    };
    
    // Use a closure to create an instance of EngineGui
    eframe::run_native(
        "Rust 2D Game Engine",
        options,
        Box::new(|_cc| Box::new(gui::EngineGui::default())),
    )
}