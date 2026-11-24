use crate::state::GuiState;
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
            .on_hover_text("Ctrl+Z (Cmd+Z on macOS)")
            .clicked()
        {
            gui_state.undo();
            ui.close();
        }
        if ui
            .add_enabled(can_redo, egui::Button::new("Redo"))
            .on_hover_text("Ctrl+Y or Ctrl+Shift+Z (Cmd+Shift+Z on macOS)")
            .clicked()
        {
            gui_state.redo();
            ui.close();
        }
    }
}
