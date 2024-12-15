use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};
use crate::gui::gui_state::{GuiState, SelectedItem};

pub struct FileSystem {
    search_query: String,
    selected_file: Option<PathBuf>,
}

impl FileSystem {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            selected_file: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) -> Option<(PathBuf, String)> {
        // Check if the project path exists
        let root_path = PathBuf::from(&gui_state.project_path);
        if !root_path.exists() {
            ui.label("Project path does not exist. Please create or open a project.");
            return None;
        }

        // Search bar
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_query);
        });

        ui.separator();

        // Render the file tree
        self.render_file_tree(ui, &root_path, 0, gui_state);

        // Return both the path and content if a Rust file was clicked
        if let Some((path, content)) = self.try_read_code_file() {
            return Some((path, content));
        }
        None
    }

    fn render_file_tree(&mut self, ui: &mut egui::Ui, path: &Path, depth: usize, gui_state: &mut GuiState) {

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

                egui::CollapsingHeader::new(format!("📁 {}", folder_name))
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
                    ui.add_space(depth as f32 * 10.0);

                    let selected = self
                        .selected_file
                        .as_ref()
                        .map_or(false, |selected_path| selected_path == &file_path);

                    let response = ui.selectable_label(selected, format!("📄 {}", file_name));

                    if response.clicked() {
                        self.selected_file = Some(file_path.clone());
                        gui_state.selected_item = SelectedItem::File(file_path.clone());
                        println!("Selected file: {}", file_name);
                    }

                    // Handle right-click context menu
                    response.context_menu(|ui| {
                        if ui.button("Delete").clicked() {
                            // TODO: check if it has references, display a popup shows "failed to remove"
                            if let Err(err) = fs::remove_file(&file_path) {
                                println!("Failed to delete file: {}", err);
                            } else {
                                println!("Deleted file: {}", file_name);
                                // Reset selected item if the deleted file was selected
                                if matches!(&gui_state.selected_item, SelectedItem::File(selected_path) if selected_path == &file_path) {
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
                    "wav", "mp3", "ogg",         // Sound files
                    "ttf", "otf",                // Font files
                    "lua",                       // Script files
                ];
                valid_extensions.contains(&ext.to_lowercase().as_str())
            }
            None => false,
        }
    }

    fn try_read_code_file(&self) -> Option<(PathBuf, String)> {
        if let Some(path) = &self.selected_file {
            // for rs and lua files
            if path.extension()?.to_str()? == "rs" || path.extension()?.to_str()? == "lua" {
                return fs::read_to_string(path)
                    .ok()
                    .map(|content| (path.clone(), content));
            }
        }
        None
    }
}
