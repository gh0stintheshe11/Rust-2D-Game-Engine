//! Project files: metadata (`project.epm`) and the scene hierarchy
//! (`scenes/scene_manager.json`).
//!
//! A project is a folder containing `project.epm`. Scaffolding new projects,
//! importing assets and building games are editor features
//! (`editor/src/project/`).

use crate::scene::SceneManager;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

mod scene_io;

static PROJECT_PATH: RwLock<Option<String>> = RwLock::new(None);

/// Name of the file that marks a folder as a project.
pub const PROJECT_FILE_NAME: &str = "project.epm";

/// Contents of `project.epm`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectMetadata {
    pub project_name: String,
    #[serde(default = "default_version")]
    pub version: String,
    /// The folder the project was opened from. Never written to the file:
    /// it would tie the project to one machine.
    #[serde(skip_serializing, default)]
    pub project_path: String,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

/// A project as loaded from disk.
#[derive(Debug)]
pub struct LoadedProject {
    pub metadata: ProjectMetadata,
    pub scene_manager: SceneManager,
}

/// Reading and writing project files. All functions are associated
/// functions; the only state is the "current project" path that scripts
/// resolve asset paths against.
pub struct ProjectManager;

impl ProjectManager {
    /// Set the current project, which scripts resolve asset paths against.
    pub fn set_project_path(path: String) {
        let mut project_path = PROJECT_PATH.write().unwrap_or_else(|e| e.into_inner());
        *project_path = Some(path);
    }

    /// The current project's folder, if any.
    pub fn get_project_path() -> Option<String> {
        PROJECT_PATH
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Resolve a project-relative asset path (as scripts write them, e.g.
    /// `assets/sounds/hit.ogg`) against the open project's root. Absolute
    /// paths are returned unchanged, and so is everything when no project
    /// is open.
    pub fn resolve_asset_path(path: &str) -> PathBuf {
        let path = Path::new(path);
        match Self::get_project_path() {
            Some(root) if path.is_relative() => Path::new(&root).join(path),
            _ => path.to_path_buf(),
        }
    }

    /// Read `project.epm`. `project_path` is set to where the project was
    /// opened from; nothing is written to disk.
    pub fn load_project(project_path: &Path) -> Result<ProjectMetadata, String> {
        let file_path = project_path.join(PROJECT_FILE_NAME);
        let json = fs::read_to_string(&file_path)
            .map_err(|e| format!("Can't read {}: {}", file_path.display(), e))?;
        let mut metadata: ProjectMetadata = serde_json::from_str(&json)
            .map_err(|e| format!("Invalid {}: {}", file_path.display(), e))?;
        metadata.project_path = project_path.to_string_lossy().into_owned();
        Ok(metadata)
    }

    /// Write `project.epm`.
    pub fn save_project(project_path: &Path, metadata: &ProjectMetadata) -> Result<(), String> {
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
        write_atomically(&project_path.join(PROJECT_FILE_NAME), &json)
    }

    /// Load a project: metadata and scenes. Makes it the current project.
    pub fn load_project_full(project_path: &Path) -> Result<LoadedProject, String> {
        if !Self::is_valid_project_directory(project_path) {
            return Err(format!(
                "{} is not a project: {} is missing",
                project_path.display(),
                PROJECT_FILE_NAME
            ));
        }
        let metadata = Self::load_project(project_path)?;
        let scene_manager = Self::load_scene_hierarchy(project_path)?;
        Self::set_project_path(metadata.project_path.clone());
        Ok(LoadedProject {
            metadata,
            scene_manager,
        })
    }

    /// Save metadata and scenes.
    pub fn save_project_full(
        project_path: &Path,
        metadata: &ProjectMetadata,
        scene_manager: &SceneManager,
    ) -> Result<(), String> {
        Self::save_project(project_path, metadata)?;
        Self::save_scene_hierarchy(project_path, scene_manager)?;
        Ok(())
    }

    /// Whether `path` is a project folder (contains `project.epm`). Asset
    /// folders are optional: version control drops empty folders, and the
    /// editor creates them when importing.
    pub fn is_valid_project_directory(path: &Path) -> bool {
        path.join(PROJECT_FILE_NAME).is_file()
    }
}

/// Replace `path`'s contents without ever leaving a truncated file behind:
/// write a temporary file next to it, then rename it over the original.
pub(crate) fn write_atomically(path: &Path, contents: &str) -> Result<(), String> {
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("Invalid file path {}", path.display()))?;
    let mut temp_name = file_name.to_os_string();
    temp_name.push(".tmp");
    let temp_path = path.with_file_name(temp_name);

    fs::write(&temp_path, contents)
        .and_then(|()| fs::rename(&temp_path, path))
        .map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            format!("Failed to write {}: {}", path.display(), e)
        })
}
