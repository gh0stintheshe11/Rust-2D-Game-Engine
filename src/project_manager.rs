use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process::Command;

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectMetadata {
    pub project_name: String,
    pub version: String,
    pub project_path: String,
    pub default_scene: String,
}

pub struct ProjectManager;

impl ProjectManager {
    pub fn create_project(project_path: &Path) -> Result<(), String> {
        let project_name = project_path.file_name()
            .and_then(|name| name.to_str())
            .ok_or("Invalid project path")?;

        let metadata = ProjectMetadata {
            project_name: project_name.to_string(),
            version: "1.0.0".to_string(),
            project_path: project_path.to_str().unwrap().to_string(),
            default_scene: "main.scene".to_string(),
        };

        Self::create_folder_structure(project_path)?;
        Self::create_metadata_file(project_path, &metadata)?;
        Self::create_main_file(project_path, project_name)?;

        Ok(())
    }

    fn create_folder_structure(base_path: &Path) -> Result<(), String> {
        let folders = [
            "assets/images",
            "assets/sounds",
            "assets/fonts",
            "scenes",
            "scripts",
            "src",  // For Rust source files
        ];

        for folder in &folders {
            let path = base_path.join(folder);
            fs::create_dir_all(&path)
                .map_err(|e| format!("Failed to create folder {}: {}", path.display(), e))?;
        }

        Ok(())
    }

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

    fn create_main_file(base_path: &Path, project_name: &str) -> Result<(), String> {
        let src_path = base_path.join("src");
        let main_path = src_path.join("main.rs");
        
        let main_content = format!(
            r#"use my_game_engine::game_runtime;
use my_game_engine::script_interpreter::LuaScriptEngine;

fn main() {{
    println!("Starting {}...");
    
    // Initialize the game runtime
    let mut runtime = game_runtime::GameRuntime::new();
    
    // Initialize Lua script engine
    let script_engine = LuaScriptEngine::new();
    
    // Load the default scene
    runtime.load_default_scene();
    
    // Start the game loop
    runtime.run();
}}
"#,
            project_name
        );

        fs::write(&main_path, main_content)
            .map_err(|e| format!("Failed to create main.rs: {}", e))?;

        // Create Cargo.toml
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

    pub fn load_project(project_path: &Path) -> Result<ProjectMetadata, String> {
        let file_path = project_path.join("project.json");
        let file = File::open(&file_path)
            .map_err(|e| format!("Failed to open file: {}", e))?;
        let metadata: ProjectMetadata = serde_json::from_reader(file)
            .map_err(|e| format!("Failed to read metadata: {}", e))?;

        Ok(metadata)
    }

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

        // Copy assets to target directory
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

        // Determine the target directory based on asset type
        let target_dir = match asset_type {
            AssetType::Image => project_path.join("assets/images"),
            AssetType::Sound => project_path.join("assets/sounds"),
            AssetType::Font => project_path.join("assets/fonts"),
            AssetType::Script => project_path.join("scripts"),
        };

        // Get the filename from the asset path
        let file_name = asset_path.file_name()
            .ok_or("Invalid asset path")?
            .to_str()
            .ok_or("Invalid asset filename")?;

        // Create the target path
        let target_path = target_dir.join(file_name);

        // Check if file already exists
        if target_path.exists() {
            return Err(format!(
                "Asset '{}' already exists in the project. Please rename the file or remove the existing one.",
                file_name
            ));
        }

        // Copy the asset file
        fs::copy(asset_path, &target_path)
            .map_err(|e| format!("Failed to copy asset: {}", e))?;

        // Return the relative path from project root
        Ok(target_path.strip_prefix(project_path)
            .map_err(|e| format!("Failed to get relative path: {}", e))?
            .to_string_lossy()
            .into_owned())
    }
}

#[derive(Debug)]
pub enum AssetType {
    Image,
    Sound,
    Font,
    Script,
}

impl AssetType {
    // Valid extensions for each asset type
    fn valid_extensions(&self) -> &[&str] {
        match self {
            AssetType::Image => &["png", "jpg", "jpeg", "gif"],
            AssetType::Sound => &["wav", "mp3", "ogg"],
            AssetType::Font => &["ttf", "otf"],
            AssetType::Script => &["lua"],  // For now just Lua scripts
        }
    }
}
