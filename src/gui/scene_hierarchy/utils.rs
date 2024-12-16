use crate::gui::gui_state::GuiState;
use crate::project_manager::ProjectManager;
use std::path::Path;
use crate::ecs::ResourceType;


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

pub fn truncate_related_path(project_path: &str, full_path: &str) -> String {
    const MAX_LENGTH: usize = 30;
    let relative_path = if full_path.starts_with(project_path) {
        full_path[project_path.len()..].trim_start_matches(std::path::MAIN_SEPARATOR).to_string()
    } else {
        full_path.to_string()
    };

    truncate_path(&relative_path)
}

pub fn is_valid_asset_file(path: &Path) -> bool {
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

pub fn resource_type_from_extension(path: &Path) -> Option<ResourceType> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => match ext.to_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "gif" => Some(ResourceType::Image),
            "wav" | "mp3" | "ogg" => Some(ResourceType::Sound),
            "lua" => Some(ResourceType::Script),
            _ => None,
        },
        None => None,
    }
}