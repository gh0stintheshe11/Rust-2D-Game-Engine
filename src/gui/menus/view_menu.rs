use eframe::egui;
use crate::gui::gui_state::GuiState;

pub struct ViewMenu;

impl ViewMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        ui.menu_button("Appearance", |ui| {
            if ui.radio_value(&mut gui_state.dark_mode, true, "🌙 Dark Mode").clicked() {
            }
            if ui.radio_value(&mut gui_state.dark_mode, false, "☀ Light Mode").clicked() {
            }
        });
    }
}
