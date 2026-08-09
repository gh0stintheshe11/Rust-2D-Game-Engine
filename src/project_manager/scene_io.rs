use std::fs;
use std::path::{Path, PathBuf};

use super::ProjectManager;
use crate::ecs::{Entity, SceneManager};

/// Asset/script paths are stored **relative to the project root** (with `/`
/// separators) in `scene_manager.json`, and resolved back to absolute paths
/// against the opened project's location at load time. This keeps projects
/// portable across machines and operating systems.
///
/// In memory, entities always hold absolute paths (renderer/audio/physics
/// open them directly).
impl ProjectManager {
    pub fn save_scene_hierarchy(
        project_path: &Path,
        scene_manager: &SceneManager,
    ) -> Result<(), String> {
        let scene_file = project_path.join("scenes").join("scene_manager.json");

        // Serialize a copy whose resource paths are relative to the project
        // root, so the file stays machine-independent.
        let mut portable = scene_manager.clone();
        map_entity_paths(&mut portable, |path| {
            to_relative_string(path, project_path)
                .map(PathBuf::from)
                .unwrap_or_else(|| path.to_path_buf())
        });

        let json = serde_json::to_string_pretty(&portable)
            .map_err(|e| format!("Failed to serialize scene hierarchy: {}", e))?;

        fs::write(&scene_file, json)
            .map_err(|e| format!("Failed to write scene hierarchy: {}", e))?;

        // Update project metadata with active scene
        if let Ok(mut metadata) = Self::load_project(project_path) {
            metadata.active_scene_id = scene_manager.active_scene;
            Self::save_project(project_path, &metadata)?;
        }

        Ok(())
    }

    // Loads the scene hierarchy from scene_manager.json
    pub fn load_scene_hierarchy(project_path: &Path) -> Result<SceneManager, String> {
        let scene_file = project_path.join("scenes").join("scene_manager.json");

        // Return new scene manager if file doesn't exist
        if !scene_file.exists() {
            return Ok(SceneManager::new());
        }

        let json = fs::read_to_string(&scene_file)
            .map_err(|e| format!("Failed to read scene hierarchy: {}", e))?;

        let mut scene_manager: SceneManager = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse scene hierarchy: {}", e))?;

        // Resolve stored paths against this machine's project location
        map_entity_paths(&mut scene_manager, |path| resolve_path(path, project_path));

        Ok(scene_manager)
    }
}

/// Apply `f` to every resource path (images, sounds, script) of every entity,
/// in all scenes and in the shared-entity registry.
fn map_entity_paths(scene_manager: &mut SceneManager, mut f: impl FnMut(&Path) -> PathBuf) {
    let mut apply = |entity: &mut Entity| {
        for image in entity.images.iter_mut() {
            *image = f(image);
        }
        for sound in entity.sounds.iter_mut() {
            *sound = f(sound);
        }
        if let Some(script) = entity.script.as_mut() {
            *script = f(script);
        }
    };

    for (_, scene) in scene_manager.scenes.iter_mut() {
        for (_, entity) in scene.entities.iter_mut() {
            apply(entity);
        }
    }
    for (_, entity) in scene_manager.shared_entities.iter_mut() {
        apply(entity);
    }
}

/// Express `path` relative to `project_path`, using `/` separators so the
/// stored form is identical on every OS. Returns None if the path isn't
/// under the project root.
fn to_relative_string(path: &Path, project_path: &Path) -> Option<String> {
    let relative = path.strip_prefix(project_path).ok()?;
    let joined = relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");
    Some(joined)
}

/// Turn a stored path into an absolute one for this machine.
///
/// - Relative paths (the current format) are joined onto the project root.
/// - Absolute paths (legacy scene files) are remapped by locating the
///   `assets/` directory in the stored string, so projects saved on another
///   machine still open.
fn resolve_path(stored: &Path, project_path: &Path) -> PathBuf {
    if stored.is_relative() {
        return project_path.join(stored);
    }

    let original = stored.to_string_lossy();
    if let Some(pos) = original.rfind("/assets/") {
        return project_path.join(&original[pos + 1..]);
    }

    stored.to_path_buf()
}
