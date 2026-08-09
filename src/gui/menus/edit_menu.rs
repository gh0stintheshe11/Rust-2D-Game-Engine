use crate::gui::gui_state::GuiState;
use crate::gui::scene_hierarchy::utils;
use eframe::egui;

pub struct EditMenu;

impl EditMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        let can_undo = gui_state.undo_stack.can_undo();
        let can_redo = gui_state.undo_stack.can_redo();

        if ui
            .add_enabled(can_undo, egui::Button::new("Undo"))
            .on_hover_text("Ctrl+Z")
            .clicked()
        {
            utils::perform_undo(gui_state);
            ui.close();
        }
        if ui
            .add_enabled(can_redo, egui::Button::new("Redo"))
            .on_hover_text("Ctrl+Y")
            .clicked()
        {
            utils::perform_redo(gui_state);
            ui.close();
        }
    }
}
