use crate::engine_gui::EngineGui;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, File};
use std::io;
use std::io::Write;
use std::path::Path;
use std::process::Command;

// Project metadata structure for project.json
#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectMetadata {
    pub project_name: String,
    pub version: String,
    pub project_path: String,
}

impl ProjectMetadata {
    // Convert metadata to JSON format
    fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap()
    }
}

// File management system struct
pub struct FileManagement;

impl FileManagement {
    // Function to create a new project at the specified path
    pub fn create_project(project_name: &str, project_path: &str, engine_gui: &mut EngineGui) {
        let base_path = format!("{}/{}", project_path, project_name);

        // Create main project folder
        FileManagement::create_folder(&base_path, engine_gui);

        // Create subfolders
        FileManagement::create_folder(&format!("{}/assets", base_path), engine_gui);
        FileManagement::create_folder(&format!("{}/assets/images", base_path), engine_gui);
        FileManagement::create_folder(&format!("{}/assets/sounds", base_path), engine_gui);
        FileManagement::create_folder(&format!("{}/assets/fonts", base_path), engine_gui);
        FileManagement::create_folder(&format!("{}/assets/videos", base_path), engine_gui);
        FileManagement::create_folder(&format!("{}/entities", base_path), engine_gui);
        FileManagement::create_folder(&format!("{}/scripts", base_path), engine_gui);
        FileManagement::create_folder(&format!("{}/scenes", base_path), engine_gui);

        // Create project.json
        let metadata = ProjectMetadata {
            project_name: project_name.to_string(),
            version: "1.0.0".to_string(),
            project_path: base_path.to_string(),
        };

        FileManagement::create_project_file(&base_path, &metadata, engine_gui);
        engine_gui.print_to_terminal(&format!(
            "Project '{}' created successfully at {}!",
            project_name, project_path
        ));
    }

    // Helper function to create folders
    fn create_folder(path: &str, engine_gui: &mut EngineGui) {
        if !Path::new(path).exists() {
            fs::create_dir_all(path).expect("Failed to create folder.");
            engine_gui.print_to_terminal(&format!("Created folder: {}", path));
        }
    }

    // Create project.json file
    fn create_project_file(
        base_path: &str,
        metadata: &ProjectMetadata,
        engine_gui: &mut EngineGui,
    ) {
        let file_path = format!("{}/project.json", base_path);
        let mut file = File::create(&file_path).expect("Failed to create project.json.");
        file.write_all(metadata.to_json().as_bytes())
            .expect("Failed to write to project.json.");
        engine_gui.print_to_terminal("Created project.json with metadata.");
    }

    // Function to check if the project path is valid
    pub fn is_valid_project_path(project_path: &str) -> bool {
        // check if the path is a directory and have project.json
        if Path::new(project_path).exists()
            && Path::new(project_path).is_dir()
            && Path::new(&format!("{}/project.json", project_path)).exists()
        {
            return true;
        }
        return false;
    }

    pub fn list_files_in_folder(folder_path: &str, engine_gui: &mut EngineGui) -> Vec<String> {
        if Path::new(folder_path).exists() {
            match fs::read_dir(folder_path) {
                Ok(read_dir) => read_dir
                    .filter_map(|entry| entry.ok().map(|e| e.file_name().into_string().unwrap()))
                    .collect(),
                Err(_) => {
                    engine_gui
                        .print_to_terminal(&format!("Failed to read folder: {}", folder_path));
                    vec![] // Return an empty vector in case of error
                }
            }
        } else {
            engine_gui.print_to_terminal(&format!("Folder does not exist: {}", folder_path));
            vec![] // Return empty vector if folder does not exist
        }
    }

    pub fn read_project_metadata(project_path: &str) -> ProjectMetadata {
        // load the project.json file to the ProjectMetadata struct
        let file_path = format!("{}/project.json", project_path);
        let file = File::open(file_path).expect("Failed to open project.json.");
        let metadata = serde_json::from_reader(file).expect("Failed to read project.json.");
        return metadata;
    }

    pub fn save_to_file(content: &str, file_path: &str) -> Result<(), String> {
        // Check if the directory exists
        let path = Path::new(file_path);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "Failed to create directory for file: {}. Error: {}",
                        file_path, e
                    )
                })?;
            }
        }

        let mut file = File::create(path)
            .map_err(|e| format!("Failed to create file: {}. Error: {}", file_path, e))?;
        file.write_all(content.as_bytes())
            .map_err(|e| format!("Failed to write to file: {}. Error: {}", file_path, e))?;

        Ok(())
    }

    pub fn delete_file(file_path: &str) -> Result<(), String> {
        match std::fs::remove_file(file_path) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to delete file '{}': {}", file_path, e)),
        }
    }

    // Extract ID from file name, assuming file name has pattern "xxx_<id>.json
    pub fn extract_id_from_file(file_name: &str) -> Option<usize> {
        // Split the file name by `_` and parse the last part as an ID
        if let Some(last_part) = file_name.rsplit('_').next() {
            last_part
                .split('.')
                .next() // In case of extension like ".json", remove it
                .and_then(|id_str| id_str.parse::<usize>().ok())
        } else {
            None
        }
    }

    // Load content from file
    pub fn load_file_content(file_path: &str) -> Result<String, String> {
        fs::read_to_string(file_path)
            .map_err(|err| format!("Failed to load content from file '{}': {}", file_path, err))
    }

    /// Import asset into project (copy asset to corresponding folder)
    pub fn import_asset(original_path: &str, dest_folder: &str) -> Result<String, String> {
        // Get the file name
        let file_path = Path::new(original_path);
        if let Some(file_name) = file_path.file_name() {
            let dest_path = Path::new(dest_folder).join(file_name);

            // Copy the file to the destination
            fs::copy(&file_path, &dest_path)
                .map(|_| format!("File imported successfully to '{}'.", dest_path.display()))
                .map_err(|err| format!("Failed to copy file to '{}': {}", dest_path.display(), err))
        } else {
            Err("Failed to get the file name.".to_string())
        }
    }

    /// Copy directory recursively
    pub fn copy_dir_recursive(src: &Path, dest: &Path) -> io::Result<()> {
        if !dest.exists() {
            fs::create_dir_all(dest)?;
        }

        println!("Copying {} -> {}", src.display(), dest.display());

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let entry_path = entry.path();
            let dest_path = dest.join(entry.file_name());

            // Skip the destination folder (build) to prevent infinite recursion
            if entry_path.starts_with(&dest) {
                continue;
            }

            if entry_path.is_dir() {
                // copy directory
                FileManagement::copy_dir_recursive(&entry_path, &dest_path)?;
            } else {
                // copy file
                fs::copy(&entry_path, &dest_path)?;
            }
        }
        Ok(())
    }

    /// Copy project folders and files
    pub fn copy_project_files(
        project_path: &str,
        build_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let project_assets = Path::new(project_path).join("assets");
        let project_entities = Path::new(project_path).join("entities");
        let project_scripts = Path::new(project_path).join("scripts");
        let project_scenes = Path::new(project_path).join("scenes");

        let build_assets = Path::new(build_path).join("assets");
        let build_entities = Path::new(build_path).join("entities");
        let build_scripts = Path::new(build_path).join("scripts");
        let build_scenes = Path::new(build_path).join("scenes");

        if project_assets.exists() {
            FileManagement::copy_dir_recursive(&project_assets, &build_assets)?;
        }
        if project_entities.exists() {
            FileManagement::copy_dir_recursive(&project_entities, &build_entities)?;
        }
        if project_scripts.exists() {
            FileManagement::copy_dir_recursive(&project_scripts, &build_scripts)?;
        }
        if project_scenes.exists() {
            FileManagement::copy_dir_recursive(&project_scenes, &build_scenes)?;
        }
        Ok(())
    }

    /// Build project by copying folders and files to build folder in project path and run cargo build --release
    pub fn build_and_run_project(
        project_path: &str,
        engine_gui: &mut EngineGui,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let build_path = format!("{}/build", project_path);

        let metadata = FileManagement::read_project_metadata(project_path);
        let project_name = metadata.project_name;
        let project_version = metadata.version;

        // Clear files in build folder
        if Path::new(&build_path).exists() {
            engine_gui
                .print_to_terminal(&format!("Removing existing build folder: {}", build_path));
            fs::remove_dir_all(&build_path)?;
        }

        if !Path::new(&build_path).exists() {
            fs::create_dir_all(&build_path)?;
        }

        fs::create_dir_all(&build_path)?;

        // Copy toml file
        let working_path = env::current_dir()?;
        let toml_file = format!("{}/Cargo.toml", working_path.display());
        engine_gui.print_to_terminal(&format!(
            "Copying Cargo.toml from {} to {}",
            toml_file, build_path
        ));
        let copied_toml_path = format!("{}/Cargo.toml", build_path);
        fs::copy(&toml_file, &copied_toml_path)?;

        FileManagement::modify_cargo_toml(&copied_toml_path, &project_name, &project_version)?;

        // copy required source files
        let working_src_path = format!("{}/src", working_path.display());
        let build_src_path = format!("{}/src", build_path);

        engine_gui.print_to_terminal(&format!(
            "Creating src folder in build path: {}",
            build_src_path
        ));
        fs::create_dir_all(&build_src_path)?;

        let required_files = [
            "ecs.rs",
            "game_runtime.rs",
            "audio_engine.rs",
            "input_handler.rs",
            "physics_engine.rs",
            "render_engine.rs",
            "project_manager.rs",
            "engine_gui.rs",
            "script_interpreter.rs",
            "shader.wgsl",
            "lib.rs",
        ];

        for file in required_files.iter() {
            let src_file = format!("{}/{}", working_src_path, file);
            let dest_file = format!("{}/{}", build_src_path, file);

            engine_gui.print_to_terminal(&format!("Copying {} to {}", src_file, dest_file));

            if Path::new(&src_file).exists() {
                fs::copy(&src_file, &dest_file)?;
            } else {
                println!("File {} does not exist in source path.", src_file);
            }
        }

        // Update `lib.rs` to exclude gui files
        let lib_file = format!("{}/lib.rs", build_src_path);
        if Path::new(&lib_file).exists() {
            engine_gui.print_to_terminal(&format!(
                "Updating lib.rs file to exclude files: {}",
                lib_file
            ));
            let lib_content = fs::read_to_string(&lib_file)?;
            let filtered_content = FileManagement::filter_lib_file(&lib_content);
            fs::write(&lib_file, filtered_content)?;
        }

        // Create main.rs for game project
        FileManagement::create_main_file(&build_src_path, &project_name)?;

        // Copy assets
        engine_gui.print_to_terminal("Copying assets from project path to build path.");
        FileManagement::copy_dir_recursive(&Path::new(&project_path), &Path::new(&build_path))?;

        // Run `cargo build --release` in build folder
        let status = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir(&build_path)
            .status()?;

        if !status.success() {
            return Err("Project build failed".into());
        }

        // Move the executable file to the root of the build folder
        let exe_path = format!("{}/target/release/{}", build_path, project_name);
        let dest_exe_path = format!("{}/{}", build_path, project_name);
        fs::rename(&exe_path, &dest_exe_path)?;

        println!(
            "Build completed successfully! Executable is in {}",
            dest_exe_path
        );

        // Run the built executable
        let status = Command::new(&dest_exe_path)
            .current_dir(&build_path)
            .status()?;

        if !status.success() {
            return Err("Failed to run the built executable".into());
        }

        Ok(())
    }

    /// Modify lib.rs file to exclude modules that are no need to build and run the game
    fn filter_lib_file(original: &str) -> String {
        // List of keywords or lines to exclude
        let excluded_lines = ["mod gui"];

        original
            .lines()
            .filter(|line| !excluded_lines.iter().any(|exclude| line.contains(exclude)))
            .collect::<Vec<&str>>()
            .join("\n")
    }

    /// Update the game project Cargo.toml file to the game project's name and version
    fn modify_cargo_toml(
        toml_path: &str,
        project_name: &str,
        project_version: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = fs::read_to_string(toml_path)?;
        let mut lines: Vec<String> = content.lines().map(|line| line.to_string()).collect();

        for line in &mut lines {
            if line.starts_with("name =") {
                *line = format!("name = \"{}\"", project_name);
            } else if line.starts_with("version =") {
                *line = format!("version = \"{}\"", project_version);
            }
        }

        let updated_content = lines.join("\n");
        fs::write(toml_path, updated_content)?;

        Ok(())
    }

    /// Create main.rs file for game project
    pub fn create_main_file(
        build_src_path: &str,
        project_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let main_file_path = format!("{}/main.rs", build_src_path);

        let main_content = format!(
            r#"
use {project_name}::game_runtime;

fn main() {{
    println!("Starting the game...");
    game_runtime::run();
}}
"#,
            project_name = project_name
        );

        fs::write(&main_file_path, main_content)?;
        println!("Generated main.rs in {}", main_file_path);

        Ok(())
    }
}
