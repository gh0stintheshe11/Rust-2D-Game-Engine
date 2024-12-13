use eframe::egui;
use crate::gui::gui_state::GuiState;

pub struct Inspector {
}

impl Inspector {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        ui.label("Properties will go here");
    }
}