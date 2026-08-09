use crate::gui::gui_state::GuiState;
use crate::project_manager::ProjectManager;

pub fn save_project(gui_state: &GuiState) {
    if let (Some(scene_manager), Some(project_metadata)) =
        (&gui_state.scene_manager, &gui_state.project_metadata)
    {
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
