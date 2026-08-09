use crate::gui::gui_state::GuiState;
use eframe::egui;

pub struct ViewMenu;

impl ViewMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        // Note: the caller already opens the "View" menu; don't nest another one
        ui.menu_button("Appearance", |ui| {
            ui.radio_value(&mut gui_state.dark_mode, true, "🌙 Dark Mode");
            ui.radio_value(&mut gui_state.dark_mode, false, "☀ Light Mode");
        });

        ui.menu_button("Panels", |ui| {
            // Direct panel toggles
            ui.checkbox(
                &mut gui_state.show_hierarchy_filesystem,
                "Hierarchy/File Panel",
            );
            ui.checkbox(&mut gui_state.show_inspector, "Inspector Panel");
            ui.checkbox(&mut gui_state.show_console, "Console Panel");
        });

        ui.checkbox(&mut gui_state.show_debug_overlay, "Debug Overlay");
    }
}
