use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use super::{LoadedProject, ProjectManager, ProjectMetadata};
use crate::ecs::SceneManager;

impl ProjectManager {
    pub fn create_project(project_path: &Path) -> Result<LoadedProject, String> {
        // Extract project name from path
        let project_name = project_path.file_name()
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

        Self::set_project_path(metadata.project_path.clone());

        // Set up project structure and files
        Self::create_folder_structure(project_path)?;
        Self::create_metadata_file(project_path, &metadata)?;
        Self::create_main_file(project_path, project_name)?;

        // Initialize and save empty scene hierarchy
        let scene_manager = SceneManager::new();
        Self::save_scene_hierarchy(project_path, &scene_manager)?;

        // Return LoadedProject just like load_project_full does
        Ok(LoadedProject {
            metadata,
            scene_manager,
        })
    }

    // Creates the standard folder structure for a new project
    fn create_folder_structure(base_path: &Path) -> Result<(), String> {
        let folders = [
            "assets/images",    // For image assets (textures, sprites)
            "assets/sounds",    // For audio assets
            "assets/fonts",     // For font files
            "assets/scripts",   // For game scripts
            "scenes",           // For scene data files
            "src",             // For Rust source files
        ];

        // Create each folder in the structure
        for folder in &folders {
            let path = base_path.join(folder);
            fs::create_dir_all(&path)
                .map_err(|e| format!("Failed to create folder {}: {}", path.display(), e))?;
        }

        Ok(())
    }

    // Creates and writes the project metadata file (project.json)
    fn create_metadata_file(base_path: &Path, metadata: &ProjectMetadata) -> Result<(), String> {
        let file_path = base_path.join(Self::PROJECT_FILE_NAME);
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
        let mut file = File::create(&file_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        file.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write to file: {}", e))?;

        Ok(())
    }

    // Creates initial source files: main.rs and Cargo.toml
    fn create_main_file(base_path: &Path, project_name: &str) -> Result<(), String> {
        let src_path = base_path.join("src");
        let main_path = src_path.join("main.rs");

        let main_content = format!(
            r#"use rust_2d_game_engine::{{
    eframe,
    ecs::SceneManager,
    render_engine::RenderEngine,
    input_handler::InputHandler,
    physics_engine::PhysicsEngine,
    audio_engine::AudioEngine,
    game_runtime::{{GameRuntime, RuntimeState}},
    project_manager::ProjectManager,
}};
use std::path::{{Path, PathBuf}};
use eframe::egui;
use std::env;

fn main() -> eframe::Result<()> {{
    // Set up panic handler for safety
    std::panic::set_hook(Box::new(|panic_info| {{
        eprintln!("Game panicked: {{}}", panic_info);
    }}));

    println!("Starting {}...");

    // Set the path to the current executable
    let exe_path = env::current_exe().expect("Failed to get current executable path");
    let exe_dir = exe_path.parent().expect("Failed to get executable directory");

    env::set_current_dir(exe_dir).expect("Failed to set working directory");
    println!("Set working directory to: {{:?}}", exe_dir);

    let project_path: PathBuf = exe_dir.to_path_buf();
    println!("Resolved project path: {{:?}}", project_path);

    let mut game_runtime = GameRuntime::new(
        SceneManager::new(),
        PhysicsEngine::new(),
        RenderEngine::new(),
        InputHandler::new(),
        AudioEngine::new(),
        60, // target fps
    );

    let mut camera_width = 800.0;
    let mut camera_height = 600.0;

    ProjectManager::set_project_path(project_path.to_string_lossy().to_string());
    let scene_manager = match ProjectManager::load_scene_hierarchy(&project_path) {{
        Ok(manager) => manager,
        Err(e) => {{
            println!("Failed to load scene hierarchy: {{}}", e);
            SceneManager::new()
        }}
    }};

    if let Some(scene) = scene_manager.get_active_scene() {{
        if let Some(camera_id) = scene.default_camera {{
            if let Ok(camera_entity) = scene.get_entity(camera_id) {{
                camera_width = camera_entity.get_camera_width();
                camera_height = camera_entity.get_camera_height();
            }}
        }}
    }}

    // Set initial window size using NativeOptions
    let native_options = eframe::NativeOptions {{
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([camera_width, camera_height])
            .with_min_inner_size([camera_width, camera_height])
            .with_maximized(true),
        ..Default::default()
    }};

    game_runtime.set_scene_manager(scene_manager.clone());
    game_runtime.run();

    eframe::run_native(
        "{}",
        native_options,
        Box::new(|cc| {{
            Ok(Box::new(MyApp {{
                game_runtime,
                camera_width,
                camera_height,
            }}))
        }}),
    )
}}

struct MyApp {{
    game_runtime: GameRuntime,
    camera_width: f32,
    camera_height: f32,
}}

impl eframe::App for MyApp {{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {{
        egui::CentralPanel::default().show(ctx, |ui| {{
            let game_rect = egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(self.camera_width, self.camera_height),
            );

            let game_view_rect = ui.available_rect_before_wrap();
            self.game_runtime.update(ctx, ui, game_rect);
        }});

        ctx.request_repaint();
    }}
}}
"#,
            project_name,
            project_name
        );

        fs::write(&main_path, main_content)
            .map_err(|e| format!("Failed to create main.rs: {}", e))?;

        // Create Cargo.toml with project configuration
        let cargo_content = format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
my_game_engine = {{ path = "../path/to/engine" }}
"#,
            project_name
        );

        let cargo_path = base_path.join("Cargo.toml");
        fs::write(&cargo_path, cargo_content)
            .map_err(|e| format!("Failed to create Cargo.toml: {}", e))?;

        Ok(())
    }
}
