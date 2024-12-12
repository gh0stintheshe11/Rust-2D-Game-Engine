use eframe::egui;
use crate::gui::gui_state::GuiState;

pub struct ViewMenu;

impl ViewMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {

        let dark_mode_button = ui.selectable_label(gui_state.dark_mode, "Dark Mode");

        if dark_mode_button.clicked() {
            gui_state.dark_mode = !gui_state.dark_mode;
            self.update_theme(ctx, gui_state.dark_mode);
        }
    }

    fn update_theme(&mut self, ctx: &egui::Context, dark_mode: bool) {
        ctx.set_visuals(if dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        ctx.request_repaint();
    }
}
