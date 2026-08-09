use rust_2d_game_engine::eframe;
use rust_2d_game_engine::engine_gui::EngineGui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([800.0, 600.0])
            .with_maximized(true),
        ..Default::default()
    };

    // Run the app
    eframe::run_native(
        "Rust Game Engine",
        options,
        Box::new(|cc| Ok(Box::new(EngineGui::new(cc)))),
    )
}
