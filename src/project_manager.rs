// Required imports for file operations, serialization, and project management
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use uuid::Uuid;
use crate::ecs::SceneManager;

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

// Main project management structure
pub struct ProjectManager;

impl ProjectManager {
    // Creates a new game project at the specified path
    // Sets up folder structure, creates initial files, and initializes scene manager
    pub fn create_project(project_path: &Path) -> Result<(), String> {
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

        // Set up project structure and files
        Self::create_folder_structure(project_path)?;
        Self::create_metadata_file(project_path, &metadata)?;
        Self::create_main_file(project_path, project_name)?;

        // Initialize and save empty scene hierarchy
        let scene_manager = SceneManager::new();
        Self::save_scene_hierarchy(project_path, &scene_manager)?;

        Ok(())
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
        let file_path = base_path.join("project.json");
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
    EngineGui,
    eframe,
    ecs::SceneManager,
}};

fn main() -> eframe::Result<()> {{
    // Set up panic handler for safety
    std::panic::set_hook(Box::new(|panic_info| {{
        eprintln!("Game panicked: {{}}", panic_info);
    }}));

    println!("Starting {}...");
    
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "{}",
        native_options,
        Box::new(|cc| {{
            // Create engine with default scene manager
            let mut engine = EngineGui::new(cc);
            
            // Scene creation and entity management will be done through the UI
            // You can use the Scene Hierarchy window to:
            // - Create new scenes
            // - Add entities
            // - Configure components
            // - Manage resources
            
            Box::new(engine)
        }})
    )
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
        let file_path = project_path.join("project.json");
        let file = File::open(&file_path)
            .map_err(|e| format!("Failed to open file: {}", e))?;
        let metadata: ProjectMetadata = serde_json::from_reader(file)
            .map_err(|e| format!("Failed to read metadata: {}", e))?;

        Ok(metadata)
    }

    // Saves project metadata to project.json
    pub fn save_project(project_path: &Path, metadata: &ProjectMetadata) -> Result<(), String> {
        let file_path = project_path.join("project.json");
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
        let status = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir(project_path)
            .status()
            .map_err(|e| format!("Failed to build project: {}", e))?;

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
            
        let scene_manager: SceneManager = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse scene hierarchy: {}", e))?;

        Ok(scene_manager)
    }

    // Loads both project metadata and scene hierarchy
    pub fn load_project_full(project_path: &Path) -> Result<(ProjectMetadata, SceneManager), String> {
        let metadata = Self::load_project(project_path)?;
        let scene_manager = Self::load_scene_hierarchy(project_path)?;
        Ok((metadata, scene_manager))
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
