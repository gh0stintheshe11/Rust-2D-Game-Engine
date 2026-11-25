use std::fs;
use std::path::{Path, PathBuf};

use rust_2d_game_engine::project::{LoadedProject, ProjectManager, ProjectMetadata};
use rust_2d_game_engine::scene::SceneManager;

/// Folders every new project starts with.
const PROJECT_FOLDERS: &[&str] = &[
    "assets/images",
    "assets/sounds",
    "assets/scripts",
    "scenes",
    "src",
];

/// Create a new game project at `project_path`: folder layout, metadata, a
/// starter scene, and a cargo package that builds the standalone game. The
/// folder must not exist yet, or be empty. On success the project becomes
/// the current project.
pub fn create_project(project_path: &Path) -> Result<LoadedProject, String> {
    let project_name = project_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .ok_or("The project folder needs a name")?;
    let occupied = fs::read_dir(project_path).is_ok_and(|mut entries| entries.next().is_some());
    if occupied {
        return Err(format!(
            "{} already exists and isn't empty",
            project_path.display()
        ));
    }

    for folder in PROJECT_FOLDERS {
        let path = project_path.join(folder);
        fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create folder {}: {}", path.display(), e))?;
    }

    let metadata = ProjectMetadata {
        project_name: project_name.to_string(),
        version: "1.0.0".to_string(),
        project_path: project_path.to_string_lossy().into_owned(),
    };
    ProjectManager::save_project(project_path, &metadata)?;
    write_cargo_package(project_path, project_name)?;

    // A starter scene, so a new project is immediately playable/buildable
    let mut scene_manager = SceneManager::new();
    let scene_id = scene_manager
        .create_scene("main")
        .map_err(|e| format!("Failed to create default scene: {}", e))?;
    scene_manager
        .set_active_scene(scene_id)
        .map_err(|e| format!("Failed to activate default scene: {}", e))?;
    ProjectManager::save_scene_hierarchy(project_path, &scene_manager)?;

    ProjectManager::set_project_path(metadata.project_path.clone());
    Ok(LoadedProject {
        metadata,
        scene_manager,
    })
}

/// Write `Cargo.toml`, `src/main.rs` and `.gitignore`: the game binary is a
/// thin wrapper around the engine's standalone player.
fn write_cargo_package(base_path: &Path, project_name: &str) -> Result<(), String> {
    let write = |relative: &str, contents: String| {
        fs::write(base_path.join(relative), contents)
            .map_err(|e| format!("Failed to create {}: {}", relative, e))
    };

    write("src/main.rs", main_rs(project_name))?;
    write(
        "Cargo.toml",
        format!(
            r#"[package]
name = "{package}"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
{dependency}

# A game project is always its own workspace, even inside another one
[workspace]
"#,
            package = package_name(project_name),
            dependency = engine_dependency(engine_checkout().as_deref()),
        ),
    )?;
    // saves/: what games played in the editor save
    write(".gitignore", "/target\n/saves\n".to_string())
}

/// The game's entry point. The title is embedded as an escaped string
/// literal, so any folder name produces valid Rust.
fn main_rs(title: &str) -> String {
    format!(
        "fn main() -> rust_2d_game_engine::eframe::Result<()> {{\n    \
         rust_2d_game_engine::player::run({:?})\n}}\n",
        title
    )
}

/// The engine checkout this editor was built from, if it still exists on
/// this machine (it doesn't for downloaded release binaries).
fn engine_checkout() -> Option<PathBuf> {
    let engine = Path::new(env!("CARGO_MANIFEST_DIR")).parent()?;
    engine
        .join("src")
        .join("lib.rs")
        .is_file()
        .then(|| engine.to_path_buf())
}

/// The game's dependency on the engine: the local checkout if there is one,
/// otherwise the engine release matching this editor's version.
fn engine_dependency(checkout: Option<&Path>) -> String {
    match checkout {
        Some(path) => format!(
            "rust-2d-game-engine = {{ path = {} }}",
            toml_string(&path.to_string_lossy().replace('\\', "/"))
        ),
        None => format!(
            "rust-2d-game-engine = {{ git = {}, tag = {} }}",
            toml_string(env!("CARGO_PKG_REPOSITORY")),
            toml_string(&format!("v{}", env!("CARGO_PKG_VERSION")))
        ),
    }
}

/// A TOML basic string.
fn toml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A cargo package name derived from the project name: lowercase ASCII
/// letters, digits, `-` and `_`, not starting with a digit, and not a name
/// cargo reserves.
fn package_name(project_name: &str) -> String {
    let name: String = project_name
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
    const RESERVED: &[&str] = &[
        "test",
        "core",
        "std",
        "alloc",
        "proc_macro",
        "proc-macro",
        "self",
        "super",
        "crate",
        "build",
        "deps",
        "examples",
        "incremental",
    ];
    let starts_with_letter = name.starts_with(|c: char| c.is_ascii_lowercase());
    if starts_with_letter && !RESERVED.contains(&name.as_str()) {
        name
    } else {
        format!("game_{}", name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A path named `name` in a temp folder removed when the returned guard
    /// is dropped.
    fn temp_dir(name: &str) -> (tempfile::TempDir, PathBuf) {
        let guard = tempfile::Builder::new()
            .prefix("rust2d_scaffold_")
            .tempdir()
            .unwrap();
        let path = guard.path().join(name);
        (guard, path)
    }

    #[test]
    fn test_created_project_is_buildable_and_playable() {
        let (_guard, project) = temp_dir("My Cool Game!");
        let loaded = create_project(&project).expect("create_project");

        let cargo = fs::read_to_string(project.join("Cargo.toml")).unwrap();
        let engine = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        assert!(
            cargo.contains(&engine_dependency(Some(engine))),
            "generated Cargo.toml must point at the engine checkout:\n{}",
            cargo
        );
        assert!(cargo.contains("name = \"my_cool_game_\""), "{}", cargo);
        assert!(cargo.contains("[workspace]"));
        let main = fs::read_to_string(project.join("src/main.rs")).unwrap();
        assert!(main.contains("player::run(\"My Cool Game!\")"), "{}", main);
        assert!(project.join(".gitignore").is_file());

        // A starter scene exists and is active, so Play works immediately
        assert_eq!(loaded.scene_manager.scenes.len(), 1);
        assert!(loaded.scene_manager.active_scene.is_some());

        let reloaded = ProjectManager::load_project_full(&project).expect("reload");
        assert_eq!(reloaded.scene_manager.scenes.len(), 1);
    }

    #[test]
    fn test_existing_folders_are_not_overwritten() {
        let (_guard, project) = temp_dir("taken");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("notes.txt"), "keep me").unwrap();

        assert!(create_project(&project).is_err());
        assert!(!project.join("project.epm").exists());
        assert_eq!(
            fs::read_to_string(project.join("notes.txt")).unwrap(),
            "keep me"
        );
    }

    #[test]
    fn test_unusual_names_produce_valid_packages() {
        assert_eq!(package_name("2048"), "game_2048");
        assert_eq!(package_name("Test"), "game_test");
        assert_eq!(package_name("_hidden"), "game__hidden");
        assert_eq!(package_name("Space Game"), "space_game");

        // Titles are embedded as escaped literals: braces and quotes are fine
        assert!(main_rs("a{b}\"c").contains(r#"run("a{b}\"c")"#));
        let (_guard, project) = temp_dir("a{b}");
        create_project(&project).expect("create_project");
    }

    #[test]
    fn test_release_binaries_depend_on_the_matching_engine_release() {
        let dependency = engine_dependency(None);
        assert!(
            dependency.contains(env!("CARGO_PKG_REPOSITORY")),
            "{}",
            dependency
        );
        assert!(
            dependency.contains(&format!("tag = \"v{}\"", env!("CARGO_PKG_VERSION"))),
            "{}",
            dependency
        );
    }
}
