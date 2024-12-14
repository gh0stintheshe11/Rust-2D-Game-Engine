use eframe::egui;
use crate::project_manager::ProjectManager;
use std::path::PathBuf;
use crate::gui::gui_state::GuiState;

pub struct FileMenu {
    temp_project_path: String,
    error_message: String,
}

impl FileMenu {
    pub fn new() -> Self {
        Self {
            temp_project_path: String::new(),
            error_message: String::new(),
        }
    }

    /// Show the File menu options.
    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        if ui.button("New Project").clicked() {
            gui_state.show_new_project_popup = true;
            gui_state.show_open_project_popup = false;
            self.temp_project_path.clear();
            self.error_message.clear();
        }

        if ui.button("Open Project").clicked() {
            gui_state.show_open_project_popup = true;
            gui_state.show_new_project_popup = false;
            self.temp_project_path.clear();
            self.error_message.clear();
        }

        if ui.button("Save Project").clicked() {
            self.save_project(gui_state);
        }

        ui.separator();

        if ui.button("Exit").clicked() {
            std::process::exit(0);
        }

        self.show_active_popup(ctx, gui_state);
    }

    /// Show the active popup window.
    pub fn show_active_popup(&mut self, ctx: &egui::Context, gui_state: &mut GuiState) {
        if gui_state.show_new_project_popup {
            self.show_new_project_popup_window(ctx, gui_state);
        }
        else if gui_state.show_open_project_popup {
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
                ui.label("Project Path:");
                ui.text_edit_singleline(&mut self.temp_project_path);
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        let path = PathBuf::from(&self.temp_project_path);
                        if path.exists() {
                            self.error_message = "Error: Project path already exists.".to_string();
                        } else {
                            match ProjectManager::create_project(&path) {
                                Ok(_) => {

                                    // Load the created project
                                    match ProjectManager::load_project(&path) {
                                        Ok(metadata) => {

                                            gui_state.project_name = metadata.project_name.clone();
                                            gui_state.project_path = metadata.project_path.clone();
                                            gui_state.load_project = true;

                                            gui_state.show_new_project_popup = false;
                                            self.error_message.clear();
                                            println!("Project '{}' created and loaded successfully!", metadata.project_name);
                                        }
                                        Err(err) => {
                                            self.error_message = format!("Error loading project: {}", err);
                                        }
                                    }
                                }
                                Err(err) => {
                                    self.error_message = format!("Error creating project: {}", err);
                                }
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
                ui.label("Project Path:");
                ui.text_edit_singleline(&mut self.temp_project_path);
                ui.horizontal(|ui| {
                    if ui.button("Open").clicked() {
                        let path = PathBuf::from(&self.temp_project_path);
                        if !path.exists() {
                            self.error_message = "Error: Path does not exist.".to_string();
                        } else {
                            match ProjectManager::load_project_full(&path) {
                                Ok((metadata, scene_manager)) => {

                                    gui_state.project_name = metadata.project_name.clone();
                                    gui_state.project_path = metadata.project_path.clone();
                                    gui_state.load_project = true;

                                    gui_state.project_metadata = Some(metadata);
                                    gui_state.scene_manager = Some(scene_manager);

                                    gui_state.show_open_project_popup = false;
                                    self.temp_project_path.clear();
                                    self.error_message.clear();
                                    println!("Project '{}' loaded!", gui_state.project_name);
                                }
                                Err(err) => {
                                    self.error_message = format!("Error loading project: {}", err);
                                }
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

    // Save project
    fn save_project(&self, gui_state: &mut GuiState) {

        if !gui_state.load_project {
            println!("No project loaded to save.");
            return;
        }

        let path = PathBuf::from(&gui_state.project_path);

        match ProjectManager::load_project(&path) {
            Ok(metadata) => {
                match ProjectManager::save_project(&path, &metadata) {
                    Ok(_) => println!("Project saved successfully."),
                    Err(err) => println!("Error saving project: {}", err),
                }
            }
            Err(_) => {
                println!("No valid project loaded to save. Please load a project first.");
            }
        }

    }

}
