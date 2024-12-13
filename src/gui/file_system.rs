use eframe::egui;
use crate::gui::gui_state::GuiState;

pub struct FileSystem {
}

impl FileSystem {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        ui.label("File browser will go here");
    }
}