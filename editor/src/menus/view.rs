use crate::state::GuiState;
use eframe::egui;

/// Grid sizes offered in View > Grid, in world units.
const GRID_SIZES: [f32; 5] = [8.0, 16.0, 32.0, 64.0, 128.0];

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
        ui.menu_button("Grid", |ui| {
            ui.checkbox(&mut gui_state.snap_to_grid, "Snap to Grid")
                .on_hover_text(
                    "Dragged entities' positions are rounded to the grid; hold Shift to invert",
                );
            ui.separator();
            for size in GRID_SIZES {
                ui.radio_value(&mut gui_state.grid_size, size, format!("{} px", size));
            }
        });
        ui.checkbox(&mut gui_state.show_colliders, "Collider Outlines")
            .on_hover_text("Draw physics colliders while the game plays (never in exported games)");
    }
}
