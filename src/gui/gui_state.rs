use crate::ecs::SceneManager;
use crate::project_manager::ProjectMetadata;
use std::path::PathBuf;
use uuid::Uuid;

pub enum SelectedItem {
    Scene(Uuid),
    Entity(Uuid, Uuid), // (Scene ID, Entity ID)
    File(PathBuf),
    None,
}

pub struct GuiState {
    pub dark_mode: bool,
    pub show_new_project_popup: bool,
    pub show_open_project_popup: bool,
    pub load_project: bool,            // Track if the project should be loaded
    pub project_name: String,          // Store the project name input
    pub project_path: String,          // Store the project path input
    pub project_metadata: Option<ProjectMetadata>,
    pub scene_manager: Option<SceneManager>,
    pub selected_item: SelectedItem,
    pub show_hierarchy_filesystem: bool,
    pub show_inspector: bool,
    pub show_console: bool,
    pub show_debug_overlay: bool,
}

impl GuiState {
    pub fn new() -> Self {
        Self {
            dark_mode: true,
            show_new_project_popup: false,
            show_open_project_popup: false,
            load_project: false,
            project_name: String::new(),
            project_path: String::new(),
            project_metadata: None,
            scene_manager: None,
            selected_item: SelectedItem::None,
            show_hierarchy_filesystem: true,
            show_inspector: true,
            show_console: true,
            show_debug_overlay: false,
        }
    }

}