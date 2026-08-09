use crate::gui::gui_state::{GuiState, ScenePanelSelectedItem, SelectedItem};
use crate::logger::LOGGER;
use crate::project_manager::ProjectManager;

/// Persist the project AND record the state in the undo history.
/// Every completed editor mutation should go through here.
pub fn save_project(gui_state: &mut GuiState) {
    save_project_without_commit(gui_state);
    if let Some(scene_manager) = &gui_state.scene_manager {
        gui_state.undo_stack.commit(scene_manager);
    }
}

/// Persist the project without touching the undo history (used by
/// undo/redo themselves and by exit-time saving).
pub fn save_project_without_commit(gui_state: &GuiState) {
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

/// Restore the previous committed state (Ctrl+Z / Edit > Undo).
pub fn perform_undo(gui_state: &mut GuiState) {
    if let Some(state) = gui_state.undo_stack.undo() {
        gui_state.scene_manager = Some(state);
        // Selections may point at entities that no longer exist
        gui_state.selected_item = SelectedItem::None;
        gui_state.scene_panel_selected_item = ScenePanelSelectedItem::None;
        save_project_without_commit(gui_state);
        LOGGER.info("Undo");
    }
}

/// Re-apply the next state (Ctrl+Y / Edit > Redo).
pub fn perform_redo(gui_state: &mut GuiState) {
    if let Some(state) = gui_state.undo_stack.redo() {
        gui_state.scene_manager = Some(state);
        gui_state.selected_item = SelectedItem::None;
        gui_state.scene_panel_selected_item = ScenePanelSelectedItem::None;
        save_project_without_commit(gui_state);
        LOGGER.info("Redo");
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
