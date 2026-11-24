//! The console panel at the bottom of the editor.

use super::EditorApp;
use eframe::egui;
use rust_2d_game_engine::logger::{ConsoleMessage, ConsoleMessageType, LOGGER};

impl EditorApp {
    /// The console panel: output/debug tabs, level filter, messages.
    pub(super) fn show_console_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.selectable_label(!self.show_debug, "💬 Output").clicked() {
                self.show_debug = false;
            }
            if ui.selectable_label(self.show_debug, "🛠 Debug").clicked() {
                self.show_debug = true;
            }
            if !self.show_debug {
                egui::ComboBox::from_id_salt("console_log_level")
                    .selected_text(format!("{:?}", self.selected_log_level))
                    .show_ui(ui, |ui| {
                        for level in [
                            ConsoleMessageType::Info,
                            ConsoleMessageType::Warning,
                            ConsoleMessageType::Error,
                        ] {
                            ui.selectable_value(
                                &mut self.selected_log_level,
                                level,
                                format!("{:?}", level),
                            );
                        }
                    });
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear").clicked() {
                    LOGGER.clear();
                }
            });
        });
        ui.separator();

        let level = if self.show_debug {
            ConsoleMessageType::Debug
        } else {
            self.selected_log_level
        };
        show_console_messages(ui, &self.console_messages, level);
    }
}

/// The messages at `level` (and more severe ones, except for the separate
/// Debug view). Only the rows in view are laid out.
fn show_console_messages(
    ui: &mut egui::Ui,
    messages: &[ConsoleMessage],
    level: ConsoleMessageType,
) {
    let visible: Vec<&ConsoleMessage> = messages
        .iter()
        .filter(|message| {
            if level == ConsoleMessageType::Debug {
                message.message_type == ConsoleMessageType::Debug
            } else {
                message.message_type >= level && message.message_type != ConsoleMessageType::Debug
            }
        })
        .collect();

    let row_height = ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().item_spacing.y;
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show_rows(ui, row_height, visible.len(), |ui, rows| {
            for message in &visible[rows] {
                let (prefix, color) = match message.message_type {
                    ConsoleMessageType::Info => ("ℹ", egui::Color32::LIGHT_BLUE),
                    ConsoleMessageType::Warning => ("⚠", egui::Color32::YELLOW),
                    ConsoleMessageType::Error => ("❌", egui::Color32::RED),
                    ConsoleMessageType::Debug => ("🔧", egui::Color32::GRAY),
                };
                ui.horizontal(|ui| {
                    ui.label(format!("[{}]", message.timestamp.format("%H:%M:%S")));
                    ui.colored_label(color, prefix);
                    ui.add(egui::Label::new(&message.text).extend());
                    if message.count > 1 {
                        ui.weak(format!("×{}", message.count));
                    }
                });
            }
        });
}
