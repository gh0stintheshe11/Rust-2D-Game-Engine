use eframe::egui;
use crate::gui::gui_state::GuiState;

pub struct EditMenu;

impl EditMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, _gui_state: &mut GuiState) {
        ui.button("Undo");
        ui.button("Redo");
    }

}
