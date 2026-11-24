//! Exit confirmation.
//!
//! Scene edits are saved as they happen, so the only thing that can be lost
//! on exit is unsaved text in the script editor; the prompt only appears
//! then.

use super::EditorApp;
use crate::state::ExitRequest;
use eframe::egui;

impl EditorApp {
    /// Turn the window's close button into an exit request. Runs in
    /// `App::logic`, which also runs while the window is minimized.
    pub(super) fn intercept_close(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.gui_state.exit_request = ExitRequest::PromptOpen;
            // The prompt must be visible to be answered
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
    }

    /// The confirmation dialog and the actual exit. Triggered by File > Exit
    /// or the window's close button.
    pub(super) fn handle_exit_flow(&mut self, ctx: &egui::Context) {
        if self.gui_state.exit_request == ExitRequest::PromptOpen {
            if !self.editor_dirty {
                self.gui_state.exit_request = ExitRequest::ExitWithoutSaving;
            } else {
                let file_name = self
                    .current_edited_file
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                egui::Modal::new(egui::Id::unique("exit_prompt")).show(ctx, |ui| {
                    ui.label(format!("Save changes to {} before exiting?", file_name));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save & Exit").clicked() {
                            self.gui_state.exit_request = ExitRequest::SaveAndExit;
                        }
                        if ui.button("Discard & Exit").clicked() {
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
                if self.save_editor_if_dirty() {
                    self.close_window(ctx);
                } else {
                    // The error is in the console; stay open so nothing is lost
                    self.gui_state.exit_request = ExitRequest::None;
                }
            }
            ExitRequest::ExitWithoutSaving => self.close_window(ctx),
            _ => {}
        }
    }

    fn close_window(&mut self, ctx: &egui::Context) {
        self.game_runtime.reset();
        self.allow_close = true;
        self.gui_state.exit_request = ExitRequest::None;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}
