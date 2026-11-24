//! The console panel at the bottom of the editor.

use super::EditorApp;
use eframe::egui;
use rust_2d_game_engine::logger::{ConsoleMessage, ConsoleMessageType};

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
        });
        ui.separator();
        if self.show_debug {
            self.show_console_messages(ui, &self.console_messages, ConsoleMessageType::Debug);
        } else {
            egui::ComboBox::from_label("Log Level")
                .selected_text(format!("{:?}", self.selected_log_level))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.selected_log_level,
                        ConsoleMessageType::Info,
                        "Info",
                    );
                    ui.selectable_value(
                        &mut self.selected_log_level,
                        ConsoleMessageType::Warning,
                        "Warning",
                    );
                    ui.selectable_value(
                        &mut self.selected_log_level,
                        ConsoleMessageType::Error,
                        "Error",
                    );
                });
            self.show_console_messages(ui, &self.console_messages, self.selected_log_level);
        }
    }

    fn show_console_messages(
        &self,
        ui: &mut egui::Ui,
        console_messages: &Vec<ConsoleMessage>,
        selected_level: ConsoleMessageType,
    ) {
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show_viewport(ui, |ui, _| {
                for message in console_messages {
                    let should_show = if selected_level == ConsoleMessageType::Debug {
                        message.message_type == ConsoleMessageType::Debug
                    } else {
                        message.message_type >= selected_level
                            && message.message_type != ConsoleMessageType::Debug
                    };

                    if should_show {
                        let time_str = message.timestamp.format("%H:%M:%S").to_string();

                        let (prefix, color) = match message.message_type {
                            ConsoleMessageType::Info => ("ℹ", egui::Color32::LIGHT_BLUE),
                            ConsoleMessageType::Warning => ("⚠", egui::Color32::YELLOW),
                            ConsoleMessageType::Error => ("❌", egui::Color32::RED),
                            ConsoleMessageType::Debug => ("🔧", egui::Color32::GRAY),
                        };

                        ui.horizontal(|ui| {
                            ui.label(format!("[{}]", time_str));
                            ui.colored_label(color, prefix);
                            ui.label(&message.text);
                            if message.count > 1 {
                                ui.weak(format!("×{}", message.count));
                            }
                            ui.allocate_exact_size(
                                egui::Vec2::new(ui.available_width(), 0.0),
                                egui::Sense::hover(),
                            );
                        });
                    }
                }
            });
    }
}
