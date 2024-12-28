// Required imports for file operations, serialization, and project management
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use uuid::Uuid;
use crate::ecs::SceneManager;
use crate::logger::LOGGER;
use std::io::{BufRead, BufReader};
use strip_ansi_escapes::strip;

use std::sync::RwLock;
static PROJECT_PATH: RwLock<Option<String>> = RwLock::new(None);

// Project metadata structure that holds basic project information
// This is serialized to/from project.json
#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectMetadata {
    pub project_name: String,      // Name of the game project
    pub version: String,           // Project version (e.g., "1.0.0")
    pub project_path: String,      // Absolute path to project directory
    pub default_scene: String,     // Default scene file name
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

    // Creates a new game project at the specified path
    // Sets up folder structure, creates initial files, and initializes scene manager
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

    // Loads project metadata from project.json
    pub fn load_project(project_path: &Path) -> Result<ProjectMetadata, String> {
        let file_path = project_path.join(Self::PROJECT_FILE_NAME);
        let file = File::open(&file_path)
            .map_err(|e| format!("Failed to open file: {}", e))?;
        
        let mut metadata: ProjectMetadata = serde_json::from_reader(file)
            .map_err(|e| format!("Failed to read metadata: {}", e))?;
        
        // Always update project_path to current path
        metadata.project_path = project_path.to_str()
            .ok_or("Invalid project path")?
            .to_string();
        
        // Save the updated metadata back to file
        Self::save_project(project_path, &metadata)?;
        
        Ok(metadata)
    }

    // Saves project metadata to project.json
    pub fn save_project(project_path: &Path, metadata: &ProjectMetadata) -> Result<(), String> {
        let file_path = project_path.join(Self::PROJECT_FILE_NAME);
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
        let mut file = File::create(&file_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        file.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write to file: {}", e))?;

        Ok(())
    }

    // Builds the project using cargo and copies assets to target directory
    pub fn build_project(project_path: &Path) -> Result<(), String> {
        // Run cargo build --release
        let mut child = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .arg("--color=always")
            .current_dir(project_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start build process: {}", e))?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let stdout_thread = std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                // Print colored output to the terminal, and log plain text output to the Debug panel
                if let Ok(line) = line {
                    println!("{}", line);
                    // Strip ANSI escape codes (for color)
                    if let Ok(clean_line) = strip(line.as_bytes()) {
                        LOGGER.debug(String::from_utf8_lossy(&clean_line));
                    }
                }
            }
        });

        let stderr_thread = std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                // Print colored output to the terminal, and log plain text output to the Debug panel
                if let Ok(line) = line {
                    eprintln!("{}", line);
                    // Strip ANSI escape codes (for color)
                    if let Ok(clean_line) = strip(line.as_bytes()) {
                        LOGGER.debug(String::from_utf8_lossy(&clean_line));
                    }
                }
            }
        });

        let status = child
            .wait()
            .map_err(|e| format!("Failed to wait on build process: {}", e))?;

        stdout_thread.join().unwrap();
        stderr_thread.join().unwrap();

        if !status.success() {
            return Err("Project build failed".into());
        }

        // Copy assets to target directory for distribution
        let target_dir = project_path.join("target/release");
        let assets_dir = project_path.join("assets");
        if assets_dir.exists() {
            let target_assets = target_dir.join("assets");
            fs::create_dir_all(&target_assets)
                .map_err(|e| format!("Failed to create target assets directory: {}", e))?;

            Self::copy_directory_contents(&assets_dir, &target_assets)
                .map_err(|e| format!("Failed to copy assets: {}", e))?;
        }

        // Copy scenes to target directory
        let scenes_dir = project_path.join("scenes");
        if scenes_dir.exists() {
            let target_scenes = target_dir.join("scenes");
            fs::create_dir_all(&target_scenes)
                .map_err(|e| format!("Failed to create target scenes directory: {}", e))?;

            Self::copy_directory_contents(&scenes_dir, &target_scenes)
                .map_err(|e| format!("Failed to copy scenes: {}", e))?;
        }

        Ok(())
    }

    // Recursively copies directory contents while preserving structure
    fn copy_directory_contents(src: &Path, dst: &Path) -> std::io::Result<()> {
        if !dst.exists() {
            fs::create_dir_all(dst)?;
        }

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let dst_path = dst.join(entry.file_name());

            if ty.is_dir() {
                Self::copy_directory_contents(&entry.path(), &dst_path)?;
            } else {
                fs::copy(entry.path(), dst_path)?;
            }
        }

        Ok(())
    }

    // Imports an asset file into the project's appropriate asset directory
    pub fn import_asset(project_path: &Path, asset_path: &Path, asset_type: AssetType) -> Result<String, String> {
        // Validate file extension
        let extension = asset_path.extension()
            .and_then(|ext| ext.to_str())
            .ok_or("File has no extension")?
            .to_lowercase();

        if !asset_type.valid_extensions().contains(&extension.as_str()) {
            return Err(format!(
                "Invalid file type for {:?}. Expected one of: {:?}",
                asset_type,
                asset_type.valid_extensions()
            ));
        }

        // Determine target directory based on asset type
        let target_dir = match asset_type {
            AssetType::Image => project_path.join("assets/images"),
            AssetType::Sound => project_path.join("assets/sounds"),
            AssetType::Font => project_path.join("assets/fonts"),
            AssetType::Script => project_path.join("assets/scripts"),
        };

        // Get filename and create target path
        let file_name = asset_path.file_name()
            .ok_or("Invalid asset path")?
            .to_str()
            .ok_or("Invalid asset filename")?;

        let target_path = target_dir.join(file_name);

        // Check for duplicate files
        if target_path.exists() {
            return Err(format!(
                "Asset '{}' already exists in the project. Please rename the file or remove the existing one.",
                file_name
            ));
        }

        // Copy the asset file
        fs::copy(asset_path, &target_path)
            .map_err(|e| format!("Failed to copy asset: {}", e))?;

        // Return relative path from project root
        Ok(target_path.strip_prefix(project_path)
            .map_err(|e| format!("Failed to get relative path: {}", e))?
            .to_string_lossy()
            .into_owned())
    }

    // Saves the current scene hierarchy to scene_manager.json
    pub fn save_scene_hierarchy(project_path: &Path, scene_manager: &SceneManager) -> Result<(), String> {
        let scene_file = project_path.join("scenes").join("scene_manager.json");
        let json = serde_json::to_string_pretty(&scene_manager)
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

        // ======== update resource paths in entities ========
        let asset_paths = [
            ("images", "assets/images"),
            ("sounds", "assets/sounds"),
            ("fonts", "assets/fonts"),
            ("scripts", "assets/scripts"),
        ];

        // Helper function to update asset paths
        fn update_asset_path(original_path: &str, project_path: &Path, asset_type: &str) -> String {
            if let Some(pos) = original_path.rfind(&format!("/{}", asset_type)) {
                // Extract the relative asset path after 'assets/{type}'
                let relative_path = &original_path[pos..];
                format!("{}/{}", project_path.display(), &relative_path[1..])
            } else {
                original_path.to_string()
            }
        }

        // Update paths
        for (_, scene) in scene_manager.scenes.iter_mut() {
            for (_, entity) in scene.entities.iter_mut() {
                // Update images
                for image in entity.images.iter_mut() {
                    let updated_path = update_asset_path(image.to_str().unwrap_or(""), project_path, asset_paths[0].1);
                    *image = PathBuf::from(updated_path);
                }

                // Update sounds
                for sound in entity.sounds.iter_mut() {
                    let updated_path = update_asset_path(sound.to_str().unwrap_or(""), project_path, asset_paths[1].1);
                    *sound = PathBuf::from(updated_path);
                }

                // Update fonts
                // for font in entity.fonts.iter_mut() {
                //     let updated_path = update_asset_path(font.to_str().unwrap_or(""), project_path, asset_paths[1].1);
                //     *font = PathBuf::from(updated_path);
                // }

                // Update script
                if let Some(script) = entity.script.as_mut() {
                    let updated_path = update_asset_path(script.to_str().unwrap_or(""), project_path, asset_paths[3].1);
                    *script = PathBuf::from(updated_path);
                }
            }
        }

        Ok(scene_manager)
    }

    // Loads both project metadata and scene hierarchy
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
        scene_manager: &SceneManager
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

// Enum defining supported asset types
#[derive(Debug)]
pub enum AssetType {
    Image,  // Image files (textures, sprites)
    Sound,  // Audio files
    Font,   // Font files
    Script, // Script files (Lua)
}

impl AssetType {
    // Returns the valid file extensions for each asset type
    pub fn valid_extensions(&self) -> &[&str] {
        match self {
            AssetType::Image => &["png", "jpg", "jpeg", "gif"],
            AssetType::Sound => &["wav", "mp3", "ogg"],
            AssetType::Font => &["ttf", "otf"],
            AssetType::Script => &["lua"],  // Currently only supporting Lua scripts
        }
    }
}
