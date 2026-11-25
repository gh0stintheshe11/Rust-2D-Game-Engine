use std::fs;
use std::path::{Path, PathBuf};

use super::{ProjectManager, write_atomically};
use crate::scene::{Entity, SceneManager};

/// Asset and script paths are stored **relative to the project root** (with
/// `/` separators) in `scene_manager.json`, and resolved back to absolute
/// paths against the opened project's location at load time. This keeps
/// projects portable across machines and operating systems.
///
/// In memory, entities hold absolute paths (renderer, audio and physics open
/// them directly).
impl ProjectManager {
    pub fn save_scene_hierarchy(
        project_path: &Path,
        scene_manager: &SceneManager,
    ) -> Result<(), String> {
        let scenes_dir = project_path.join("scenes");
        fs::create_dir_all(&scenes_dir)
            .map_err(|e| format!("Failed to create {}: {}", scenes_dir.display(), e))?;

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
        write_atomically(&scenes_dir.join("scene_manager.json"), &json)
    }

    /// Load `scenes/scene_manager.json`; a project without one has no
    /// scenes yet.
    pub fn load_scene_hierarchy(project_path: &Path) -> Result<SceneManager, String> {
        let scene_file = project_path.join("scenes").join("scene_manager.json");
        if !scene_file.exists() {
            return Ok(SceneManager::new());
        }

        let json = fs::read_to_string(&scene_file)
            .map_err(|e| format!("Failed to read scene hierarchy: {}", e))?;
        let mut scene_manager: SceneManager = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse scene hierarchy: {}", e))?;

        scene_manager.repair_ids();
        // Resolve stored paths against this machine's project location
        map_entity_paths(&mut scene_manager, |path| resolve_path(path, project_path));

        Ok(scene_manager)
    }

    /// Store one entity's resource paths relative to `project_path`, the
    /// way scene files do (e.g. to copy the entity to the clipboard). Paths
    /// outside the project stay as they are.
    pub fn make_entity_paths_relative(entity: &mut Entity, project_path: &Path) {
        map_paths(entity, &mut |path| {
            to_relative_string(path, project_path)
                .map(PathBuf::from)
                .unwrap_or_else(|| path.to_path_buf())
        });
    }

    /// Resolve one entity's stored resource paths against `project_path`,
    /// the way loading a scene does.
    pub fn resolve_entity_paths(entity: &mut Entity, project_path: &Path) {
        map_paths(entity, &mut |path| resolve_path(path, project_path));
    }
}

/// Apply `f` to every resource path (images, sounds, script) of an entity.
fn map_paths(entity: &mut Entity, f: &mut impl FnMut(&Path) -> PathBuf) {
    for image in entity.images.iter_mut() {
        *image = f(image);
    }
    for sound in entity.sounds.iter_mut() {
        *sound = f(sound);
    }
    if let Some(script) = entity.script.as_mut() {
        *script = f(script);
    }
}

/// Apply `f` to every resource path of every entity, in all scenes and in
/// the shared-entity registry.
fn map_entity_paths(scene_manager: &mut SceneManager, mut f: impl FnMut(&Path) -> PathBuf) {
    for scene in scene_manager.scenes.values_mut() {
        for entity in scene.entities.values_mut() {
            map_paths(entity, &mut f);
        }
    }
    for entity in scene_manager.shared_entities.values_mut() {
        map_paths(entity, &mut f);
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
/// - Absolute paths (written by older versions, possibly on another OS) are
///   remapped by locating their `assets/` directory. Stored paths are
///   compared as strings with `/` separators, so a Windows path is
///   recognized on Unix and vice versa.
fn resolve_path(stored: &Path, project_path: &Path) -> PathBuf {
    let original = stored.to_string_lossy();
    let normalized = original.replace('\\', "/");

    if !is_absolute_on_any_os(&normalized) {
        return join_slash_path(project_path, &normalized);
    }
    if let Some(pos) = normalized.rfind("/assets/") {
        return join_slash_path(project_path, &normalized[pos + 1..]);
    }
    stored.to_path_buf()
}

/// `/unix`, `C:/windows` and `//server/share` paths.
fn is_absolute_on_any_os(path: &str) -> bool {
    let bytes = path.as_bytes();
    path.starts_with('/')
        || (bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && &bytes[1..3] == b":/")
}

fn join_slash_path(root: &Path, relative: &str) -> PathBuf {
    relative
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .fold(root.to_path_buf(), |path, part| path.join(part))
}
