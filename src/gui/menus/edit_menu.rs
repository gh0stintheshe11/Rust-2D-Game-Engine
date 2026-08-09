use crate::gui::gui_state::GuiState;
use eframe::egui;

pub struct EditMenu;

impl EditMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, _gui_state: &mut GuiState) {
        // Undo/redo is not implemented yet; show the entries disabled so the
        // menu is honest about it
        ui.add_enabled(false, egui::Button::new("Undo"))
            .on_disabled_hover_text("Not implemented yet");
        ui.add_enabled(false, egui::Button::new("Redo"))
            .on_disabled_hover_text("Not implemented yet");
    }
}
