use crate::gui::gui_state::GuiState;
use crate::project_manager::ProjectManager;
use std::path::Path;


pub fn save_project(gui_state: &GuiState) {
    if let (Some(scene_manager), Some(project_metadata)) = (
        &gui_state.scene_manager,
        &gui_state.project_metadata,
    ) {
        match ProjectManager::save_project_full(
            Path::new(&gui_state.project_path),
            project_metadata,
            scene_manager,
        ) {
            Ok(_) => println!("Project saved successfully."),
            Err(err) => println!("Error saving project: {}", err),
        }
    } else {
        println!("Error: Scene manager or project metadata is missing.");
    }
}

pub fn is_valid_identifier(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

pub fn truncate_path(path: &str) -> String {
    // Maximum length for display
    const MAX_LENGTH: usize = 30;
    if path.len() > MAX_LENGTH {
        let start = &path[..10];
        let end = &path[path.len() - 10..];
        format!("{}...{}", start, end)
    } else {
        path.to_string()
    }
}