use crate::gui::gui_state::GuiState;
use crate::project_manager::ProjectManager;
use std::path::{Path, PathBuf};
use crate::ecs::Entity;

pub fn save_project(gui_state: &GuiState) {
    if let (Some(scene_manager), Some(project_metadata)) = (
        &gui_state.scene_manager,
        &gui_state.project_metadata,
    ) {
        match ProjectManager::save_project_full(
            &gui_state.project_path,
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

pub fn truncate_path(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    const MAX_LENGTH: usize = 30;
    if path_str.len() > MAX_LENGTH {
        let start = &path_str[..10];
        let end = &path_str[path_str.len() - 10..];
        format!("{}...{}", start, end)
    } else {
        path_str.to_string()
    }
}

pub fn truncate_related_path(project_path: &Path, full_path: &Path) -> String {
    if let Ok(relative_path) = full_path.strip_prefix(project_path) {
        truncate_path(relative_path)
    } else {
        truncate_path(full_path)
    }
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

pub fn get_icon_for_file(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("");

    match extension.to_lowercase().as_str() {
        "png" | "jpg" | "jpeg" => "🖼️",
        "wav" | "mp3" | "ogg" => "🔊",
        "rs" | "lua" => "📄",
        _ => "❓"
    }
}

pub fn format_file_size(size_in_bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size_in_bytes >= GB {
        format!("{:.2} GB", size_in_bytes as f64 / GB as f64)
    } else if size_in_bytes >= MB {
        format!("{:.2} MB", size_in_bytes as f64 / MB as f64)
    } else if size_in_bytes >= KB {
        format!("{:.2} KB", size_in_bytes as f64 / KB as f64)
    } else {
        format!("{} B", size_in_bytes)
    }
}