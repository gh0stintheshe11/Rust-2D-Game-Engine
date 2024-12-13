use eframe::egui;
use crate::gui::gui_state::GuiState;

pub struct ImportMenu;

impl ImportMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        ui.button("Import Sound");
        ui.button("Import Image");
        ui.button("Import Script");
    }

}
