//! Exit confirmation.

use super::EditorApp;
use crate::state::ExitRequest;
use eframe::egui;

impl EditorApp {
    /// Exit confirmation dialog + the actual exit path. Triggered by
    /// File > Exit or the native window close button.
    pub(super) fn handle_exit_flow(&mut self, ctx: &egui::Context) {
        // Intercept the native close button so unsaved work gets a prompt
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_close
            && self.gui_state.load_project
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.gui_state.exit_request = ExitRequest::PromptOpen;
        }

        if self.gui_state.exit_request == ExitRequest::PromptOpen {
            // Nothing worth prompting about without an open project
            if !self.gui_state.load_project {
                self.gui_state.exit_request = ExitRequest::ExitWithoutSaving;
            } else {
                egui::Window::new("Exit")
                    .collapsible(false)
                    .resizable(false)
                    .order(egui::Order::Foreground)
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .show(ctx, |ui| {
                        ui.label("Save the project before exiting?");
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Save & Exit").clicked() {
                                self.gui_state.exit_request = ExitRequest::SaveAndExit;
                            }
                            if ui.button("Exit without saving").clicked() {
                                self.gui_state.exit_request = ExitRequest::ExitWithoutSaving;
                            }
                            if ui.button("Cancel").clicked() {
                                self.gui_state.exit_request = ExitRequest::None;
                            }
                        });
                    });
            }
        }

        match self.gui_state.exit_request {
            ExitRequest::SaveAndExit => {
                self.save_editor_if_dirty();
                crate::panels::hierarchy::utils::save_project_without_commit(&self.gui_state);
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ExitRequest::ExitWithoutSaving => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            _ => {}
        }
    }
}
