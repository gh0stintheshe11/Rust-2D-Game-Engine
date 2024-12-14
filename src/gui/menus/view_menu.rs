use eframe::egui;
use crate::gui::gui_state::GuiState;

pub struct ViewMenu;

impl ViewMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        ui.menu_button("View", |ui| {
            ui.menu_button("Appearance", |ui| {
                if ui.radio_value(&mut gui_state.dark_mode, true, "🌙 Dark Mode").clicked() {
                }
                if ui.radio_value(&mut gui_state.dark_mode, false, "☀ Light Mode").clicked() {
                }
            });
            
            ui.menu_button("Panels", |ui| {
                // Direct panel toggles
                ui.checkbox(&mut gui_state.show_hierarchy_filesystem, "Hierarchy/File Panel");
                ui.checkbox(&mut gui_state.show_inspector, "Inspector Panel");
                ui.checkbox(&mut gui_state.show_console, "Console Panel");
            });

            ui.checkbox(&mut gui_state.show_debug_overlay, "Debug Overlay");
        });
    }
}

