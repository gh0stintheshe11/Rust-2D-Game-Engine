use crate::state::{ExitRequest, GuiState};
use eframe::egui;
use rust_2d_game_engine::project::ProjectManager;
use std::path::PathBuf;

pub struct FileMenu {
    temp_project_path: PathBuf,
    error_message: String,
}

impl FileMenu {
    pub fn new() -> Self {
        Self {
            temp_project_path: PathBuf::new(),
            error_message: String::new(),
        }
    }

    /// Show the File menu options.
    pub fn show(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        if ui.button("New Project").clicked() {
            gui_state.show_new_project_popup = true;
            gui_state.show_open_project_popup = false;
            self.temp_project_path.clear();
            self.error_message.clear();
            ui.close();
        }

        if ui.button("Open Project").clicked() {
            gui_state.show_open_project_popup = true;
            gui_state.show_new_project_popup = false;
            self.temp_project_path.clear();
            self.error_message.clear();
            ui.close();
        }

        if ui
            .add_enabled(gui_state.project_open, egui::Button::new("Save Project"))
            .on_hover_text("Edits are saved automatically; this also saves the open script")
            .clicked()
        {
            gui_state.save_requested = true;
            ui.close();
        }

        ui.separator();

        if ui.button("Exit").clicked() {
            gui_state.exit_request = ExitRequest::PromptOpen;
            ui.close();
        }
    }

    /// Show the active popup window.
    pub fn show_active_popup(&mut self, ctx: &egui::Context, gui_state: &mut GuiState) {
        if gui_state.show_new_project_popup {
            self.show_new_project_popup_window(ctx, gui_state);
        } else if gui_state.show_open_project_popup {
            self.show_open_project_popup_window(ctx, gui_state);
        }
    }

    /// Show the "New Project" popup window.
    fn show_new_project_popup_window(&mut self, ctx: &egui::Context, gui_state: &mut GuiState) {
        egui::Window::new("Create New Project")
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.label("Project folder (created if it doesn't exist, must be empty if it does):");
                ui.horizontal(|ui| {
                    let mut path_str = self.temp_project_path.to_string_lossy().into_owned();
                    if ui.text_edit_singleline(&mut path_str).changed() {
                        self.temp_project_path = PathBuf::from(&path_str);
                    }
                    if ui.button("Browse…").clicked()
                        && let Some(parent) = rfd::FileDialog::new()
                            .set_title("Choose where to create the project")
                            .pick_folder()
                    {
                        // Suggest a subfolder inside the chosen directory
                        self.temp_project_path = parent.join("my_game");
                    }
                });

                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        match crate::project::create_project(&self.temp_project_path) {
                            Ok(loaded) => {
                                gui_state.open_project(loaded);
                                gui_state.show_new_project_popup = false;
                                self.error_message.clear();
                            }
                            Err(err) => {
                                self.error_message = format!("Error creating project: {}", err);
                            }
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        gui_state.show_new_project_popup = false;
                        self.temp_project_path.clear();
                        self.error_message.clear();
                    }
                });

                if !self.error_message.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.error_message);
                }
            });
    }

    /// Show the "Open Project" popup window.
    fn show_open_project_popup_window(&mut self, ctx: &egui::Context, gui_state: &mut GuiState) {
        egui::Window::new("Open Project")
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.label("Project folder (the one containing project.epm):");
                ui.horizontal(|ui| {
                    let mut path_str = self.temp_project_path.to_string_lossy().into_owned();
                    if ui.text_edit_singleline(&mut path_str).changed() {
                        self.temp_project_path = PathBuf::from(&path_str);
                        self.error_message.clear();
                    }
                    if ui.button("Browse…").clicked()
                        && let Some(folder) = rfd::FileDialog::new()
                            .set_title("Open a project folder")
                            .pick_folder()
                    {
                        self.temp_project_path = folder;
                        self.error_message.clear();
                    }
                });

                ui.horizontal(|ui| {
                    let has_path = !self.temp_project_path.as_os_str().is_empty();
                    if ui
                        .add_enabled(has_path, egui::Button::new("Open"))
                        .clicked()
                    {
                        match ProjectManager::load_project_full(&self.temp_project_path) {
                            Ok(loaded) => {
                                gui_state.open_project(loaded);
                                gui_state.show_open_project_popup = false;
                                self.temp_project_path.clear();
                                self.error_message.clear();
                            }
                            Err(err) => {
                                self.error_message = format!("Error loading project: {}", err);
                            }
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        gui_state.show_open_project_popup = false;
                        self.temp_project_path.clear();
                        self.error_message.clear();
                    }
                });

                if !self.error_message.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.error_message);
                }
            });
    }
}
