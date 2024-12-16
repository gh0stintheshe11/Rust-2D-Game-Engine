use crate::gui::gui_state::{GuiState, SelectedItem};
use crate::project_manager::ProjectManager;
use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};

pub struct FileSystem {
    search_query: String,
    selected_file: Option<PathBuf>,
    show_search: bool,
}

impl FileSystem {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            selected_file: None,
            show_search: false,
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        gui_state: &mut GuiState,
    ) -> Option<(PathBuf, String)> {
        // Always show header with integrated search
        egui::Frame {
            inner_margin: egui::Margin::ZERO,
            outer_margin: egui::Margin {
                left: 2.0,
                right: 6.0,
                top: 6.0,
                bottom: 0.0,
            },
            rounding: egui::Rounding::ZERO,
            shadow: eframe::epaint::Shadow::NONE,
            fill: egui::Color32::TRANSPARENT,
            stroke: egui::Stroke::NONE,
        }
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Files");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔍").clicked() {
                        self.show_search = !self.show_search;
                        if !self.show_search {
                            self.search_query.clear();
                        }
                    }

                    if self.show_search {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.search_query)
                                .desired_width(150.0)
                                .hint_text("Search files..."),
                        );
                    }
                });
            });
            ui.separator();
        });

        // File tree in scrollable area with margin
        egui::Frame {
            inner_margin: egui::Margin::ZERO,
            outer_margin: egui::Margin {
                left: 2.0,
                right: 2.0,
                top: 0.0,
                bottom: 2.0,
            },
            rounding: egui::Rounding::ZERO,
            shadow: eframe::epaint::Shadow::NONE,
            fill: egui::Color32::TRANSPARENT,
            stroke: egui::Stroke::NONE,
        }
        .show(ui, |ui| {
            if !gui_state.load_project {
                ui.label("No project opened.");
                return;
            }
            let path = gui_state.project_path.clone();
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    self.render_file_tree(ui, &path, 0, gui_state);
                });
        });

        // Return file content if selected
        self.try_read_code_file()
    }

    fn render_file_tree(
        &mut self,
        ui: &mut egui::Ui,
        path: &Path,
        depth: usize,
        gui_state: &mut GuiState,
    ) {
        if let Ok(mut entries) = fs::read_dir(path) {
            let search_query = self.search_query.to_lowercase();
            let is_filtering = !search_query.is_empty();

            // Split into folders and files, to show folders first
            let mut folders = vec![];
            let mut files = vec![];

            for entry in entries.filter_map(|e| e.ok()) {
                if entry.path().is_dir() {
                    folders.push(entry);
                } else {
                    files.push(entry);
                }
            }

            // Sort folders and files
            folders.sort_by_key(|e| e.file_name());
            files.sort_by_key(|e| e.file_name());

            // Render folders
            for folder in folders {
                let folder_path = folder.path();
                let folder_name = folder.file_name().to_string_lossy().to_string();

                egui::CollapsingHeader::new(format!("{}", folder_name))
                    .default_open(true)
                    .show(ui, |ui| {
                        self.render_file_tree(ui, &folder_path, depth + 1, gui_state);
                    });
            }

            // Render files
            for file in files {
                let file_path = file.path();
                let file_name = match file_path.file_name() {
                    Some(name) => name.to_string_lossy().to_string(),
                    None => continue,
                };

                // Apply search filter to files only
                if is_filtering && !file_name.to_lowercase().contains(&search_query) {
                    continue;
                }

                // Render files
                ui.horizontal(|ui| {
                    ui.add_space(depth as f32 * 4.0);

                    let selected = self.selected_file.as_ref().map_or(false, |selected_path| 
                        selected_path == &file_path
                    );

                    let response = ui.selectable_label(selected, format!("{}", file_name));

                    if response.clicked() {
                        self.selected_file = Some(file_path.clone());
                        gui_state.selected_item = SelectedItem::File(file_path.clone());
                        println!("Selected file: {}", file_name);
                    }

                    response.context_menu(|ui| {
                        if ui.button("Delete").clicked() {
                            if let Err(err) = fs::remove_file(&file_path) {
                                println!("Failed to delete file: {}", err);
                            } else {
                                println!("Deleted file: {}", file_name);
                                if matches!(&gui_state.selected_item, 
                                    SelectedItem::File(selected_path) 
                                    if selected_path == &file_path) 
                                {
                                    gui_state.selected_item = SelectedItem::None;
                                }
                                if self.selected_file == Some(file_path.clone()) {
                                    self.selected_file = None;
                                }
                            }
                            ui.close_menu();
                        }
                    });
                });
            }
        } else {
            ui.label("Failed to read directory.");
        }
    }

    fn is_valid_file(&self, path: &Path) -> bool {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) => {
                let valid_extensions = [
                    "png", "jpg", "jpeg", "gif", // Image files
                    "wav", "mp3", "ogg", // Sound files
                    "ttf", "otf", // Font files
                    "lua", // Script files
                ];
                valid_extensions.contains(&ext.to_lowercase().as_str())
            }
            None => false,
        }
    }

    fn try_read_code_file(&self) -> Option<(PathBuf, String)> {
        if let Some(path) = &self.selected_file {
            if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                if ext == "rs" || ext == "lua" {
                    return fs::read_to_string(path)
                        .ok()
                        .map(|content| (path.clone(), content));
                }
            }
        }
        None
    }
}
