use std::fs;
use std::path::Path;

use rust_2d_game_engine::project::{LoadedProject, ProjectManager, ProjectMetadata};
use rust_2d_game_engine::scene::SceneManager;

/// Create a new game project at `project_path`: folder layout, metadata,
/// a starter scene, and a cargo package that builds the standalone game.
pub fn create_project(project_path: &Path) -> Result<LoadedProject, String> {
    // Extract project name from path
    let project_name = project_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("Invalid project path")?;

    // Create initial project metadata
    let metadata = ProjectMetadata {
        project_name: project_name.to_string(),
        version: "1.0.0".to_string(),
        project_path: project_path.to_str().unwrap().to_string(),
        default_scene: "main.scene".to_string(),
        active_scene_id: None,
    };

    ProjectManager::set_project_path(metadata.project_path.clone());

    // Set up project structure and files
    create_folder_structure(project_path)?;
    ProjectManager::save_project(project_path, &metadata)?;
    create_cargo_package(project_path, project_name)?;

    // Initialize the scene hierarchy with a starter scene so a new project is
    // immediately playable/buildable
    let mut scene_manager = SceneManager::new();
    let scene_id = scene_manager
        .create_scene("main")
        .map_err(|e| format!("Failed to create default scene: {}", e))?;
    scene_manager
        .set_active_scene(scene_id)
        .map_err(|e| format!("Failed to activate default scene: {}", e))?;
    ProjectManager::save_scene_hierarchy(project_path, &scene_manager)?;

    Ok(LoadedProject {
        metadata,
        scene_manager,
    })
}

/// Create the standard folder structure for a new project.
fn create_folder_structure(base_path: &Path) -> Result<(), String> {
    let folders = [
        "assets/images",  // For image assets (textures, sprites)
        "assets/sounds",  // For audio assets
        "assets/fonts",   // For font files
        "assets/scripts", // For game scripts
        "scenes",         // For scene data files
        "src",            // For Rust source files
    ];

    for folder in &folders {
        let path = base_path.join(folder);
        fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create folder {}: {}", path.display(), e))?;
    }

    Ok(())
}

/// Write `Cargo.toml` and `src/main.rs`: the game binary is a thin wrapper
/// around the engine's standalone player.
fn create_cargo_package(base_path: &Path, project_name: &str) -> Result<(), String> {
    let main_content = format!(
        "fn main() -> rust_2d_game_engine::eframe::Result<()> {{\n    \
         rust_2d_game_engine::player::run({:?})\n}}\n",
        project_name
    );
    let main_path = base_path.join("src").join("main.rs");
    fs::write(&main_path, main_content).map_err(|e| format!("Failed to create main.rs: {}", e))?;

    // The engine dependency points at the engine checkout that built this
    // editor (the editor crate lives in `<engine>/editor`, baked in at
    // compile time), so a freshly created project builds out of the box on
    // this machine.
    let engine_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("Engine checkout not found")?
        .to_string_lossy()
        .replace('\\', "/");
    // Cargo package names are restricted; derive a safe one from the
    // project folder name
    let package_name: String = project_name
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cargo_content = format!(
        r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "2021"

[dependencies]
rust-2d-game-engine = {{ path = "{engine_path}" }}
# If you move this project to another machine, point the dependency at your
# engine checkout there, or use the git dependency instead:
# rust-2d-game-engine = {{ git = "https://github.com/gh0stintheshe11/Rust-2D-Game-Engine" }}

# A game project is always its own workspace, even inside another one
[workspace]
"#
    );

    let cargo_path = base_path.join("Cargo.toml");
    fs::write(&cargo_path, cargo_content)
        .map_err(|e| format!("Failed to create Cargo.toml: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_created_project_is_buildable_and_playable() {
        let parent =
            std::env::temp_dir().join(format!("rust2d_pm_scaffold_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&parent).unwrap();
        let project = parent.join("My Cool Game!");

        let loaded = create_project(&project).expect("create_project");

        // The generated Cargo.toml must reference the real engine, not a
        // placeholder, and must have a valid package name
        let cargo = fs::read_to_string(project.join("Cargo.toml")).unwrap();
        let engine_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        assert!(
            cargo.contains(&format!(
                "rust-2d-game-engine = {{ path = \"{}\" }}",
                engine_path
            )),
            "generated Cargo.toml must point at the engine that created it:\n{}",
            cargo
        );
        assert!(
            cargo.contains("name = \"my_cool_game_\""),
            "package name must be sanitized:\n{}",
            cargo
        );
        assert!(!cargo.contains("path/to/engine"), "no placeholder paths");

        // A starter scene exists and is active, so Play works immediately
        assert_eq!(loaded.scene_manager.scenes.len(), 1);
        assert!(loaded.scene_manager.active_scene.is_some());

        // The scene file round-trips
        let reloaded = ProjectManager::load_project_full(&project).expect("reload");
        assert_eq!(reloaded.scene_manager.scenes.len(), 1);
        assert!(project.join("src/main.rs").exists());
    }
}
