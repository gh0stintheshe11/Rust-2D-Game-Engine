//! Project lifecycle: metadata, save/load, scaffolding, asset import, builds.
//!
//! Split across submodules for readability; everything is re-exported here so
//! external code keeps using `crate::project_manager::*` paths.
use crate::ecs::SceneManager;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

mod assets;
mod build;
mod scaffold;
mod scene_io;

pub use assets::AssetType;

use std::sync::RwLock;
static PROJECT_PATH: RwLock<Option<String>> = RwLock::new(None);

// Project metadata structure that holds basic project information
// This is serialized to/from project.json
#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectMetadata {
    pub project_name: String,          // Name of the game project
    pub version: String,               // Project version (e.g., "1.0.0")
    pub project_path: String,          // Absolute path to project directory
    pub default_scene: String,         // Default scene file name
    pub active_scene_id: Option<Uuid>, // Currently active scene's UUID
}

// Add a new struct to represent project loading result
#[derive(Debug)]
pub struct LoadedProject {
    pub metadata: ProjectMetadata,
    pub scene_manager: SceneManager,
}

// Main project management structure
pub struct ProjectManager;

impl ProjectManager {
    // Add constant definition
    const PROJECT_FILE_NAME: &'static str = "project.epm";

    pub fn set_project_path(path: String) {
        let mut project_path_lock = PROJECT_PATH.write().unwrap();
        *project_path_lock = Some(path);
    }

    /// Get the global project path
    pub fn get_project_path() -> Option<String> {
        let project_path_lock = PROJECT_PATH.read().unwrap();
        project_path_lock.clone()
    }

    // Loads project metadata from project.epm.
    //
    // The stored project_path is ignored and replaced with the actual
    // location the project was opened from (in memory only - loading must
    // not mutate the project on disk). The path is persisted on the next
    // explicit save.
    pub fn load_project(project_path: &Path) -> Result<ProjectMetadata, String> {
        let file_path = project_path.join(Self::PROJECT_FILE_NAME);
        let file = File::open(&file_path).map_err(|e| format!("Failed to open file: {}", e))?;

        let mut metadata: ProjectMetadata =
            serde_json::from_reader(file).map_err(|e| format!("Failed to read metadata: {}", e))?;

        // Always update project_path to current path
        metadata.project_path = project_path
            .to_str()
            .ok_or("Invalid project path")?
            .to_string();

        Ok(metadata)
    }

    // Saves project metadata to project.json
    pub fn save_project(project_path: &Path, metadata: &ProjectMetadata) -> Result<(), String> {
        let file_path = project_path.join(Self::PROJECT_FILE_NAME);
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
        let mut file =
            File::create(&file_path).map_err(|e| format!("Failed to create file: {}", e))?;
        file.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write to file: {}", e))?;

        Ok(())
    }

    // Builds the project using cargo and copies assets to target directory
    pub fn load_project_full(project_path: &Path) -> Result<LoadedProject, String> {
        // First check for EPM file
        if !Self::is_valid_project_directory(project_path) {
            return Err("Not a valid project - missing project.epm file".to_string());
        }

        //valid project structure
        if !Self::validate_project_structure(project_path).is_ok() {
            return Err("Project structure is invalid".to_string());
        }

        // Load and update project metadata (this will update and save the new path)
        let metadata = Self::load_project(project_path)?;

        // Load scene manager
        let scene_manager = Self::load_scene_hierarchy(project_path)?;

        Self::set_project_path(metadata.project_path.clone());

        // Return loaded project with updated metadata
        Ok(LoadedProject {
            metadata,
            scene_manager,
        })
    }

    // Saves both project metadata and scene hierarchy
    pub fn save_project_full(
        project_path: &Path,
        metadata: &ProjectMetadata,
        scene_manager: &SceneManager,
    ) -> Result<(), String> {
        Self::save_project(project_path, metadata)?;
        Self::save_scene_hierarchy(project_path, scene_manager)?;
        Ok(())
    }

    // Modify is_valid_project_directory to be more explicit
    pub fn is_valid_project_directory(path: &Path) -> bool {
        path.join(Self::PROJECT_FILE_NAME).exists()
    }

    // Modify validate_project_structure to check EPM file first
    pub fn validate_project_structure(project_path: &Path) -> Result<(), String> {
        // First and most important check - EPM file
        if !project_path.join(Self::PROJECT_FILE_NAME).exists() {
            return Err("Not a valid project - missing project.epm file".to_string());
        }

        let required_folders = [
            "assets/images",
            "assets/sounds",
            "assets/fonts",
            "assets/scripts",
            "scenes",
            "src",
        ];

        // Then check folders
        for folder in &required_folders {
            let folder_path = project_path.join(folder);
            if !folder_path.exists() {
                return Err(format!("Required folder '{}' is missing", folder));
            }
        }

        // Finally check scene manager
        let scene_file = project_path.join("scenes").join("scene_manager.json");
        if !scene_file.exists() {
            return Err("Scene manager file is missing".to_string());
        }

        Ok(())
    }
}
