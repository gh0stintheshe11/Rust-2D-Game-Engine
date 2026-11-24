//! Rust 2D Game Engine editor.

mod app;
mod menu_bar;
mod menus;
mod panels;
mod project;
mod state;

use app::EditorApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([800.0, 600.0])
            .with_maximized(true),
        ..Default::default()
    };

    eframe::run_native(
        "Rust 2D Game Engine",
        options,
        Box::new(|cc| Ok(Box::new(EditorApp::new(&cc.egui_ctx)))),
    )
}
