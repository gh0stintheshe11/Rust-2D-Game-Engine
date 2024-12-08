use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use serde::{Serialize, Deserialize};
use crate::engine_gui::EngineGui;

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
        engine_gui.print_to_terminal(&format!("Project '{}' created successfully at {}!", project_name, project_path));
    }

    // Helper function to create folders
    fn create_folder(path: &str, engine_gui: &mut EngineGui) {
        if !Path::new(path).exists() {
            fs::create_dir_all(path).expect("Failed to create folder.");
            engine_gui.print_to_terminal(&format!("Created folder: {}", path));
        }
    }

    // Create project.json file
    fn create_project_file(base_path: &str, metadata: &ProjectMetadata, engine_gui: &mut EngineGui) {
        let file_path = format!("{}/project.json", base_path);
        let mut file = File::create(&file_path).expect("Failed to create project.json.");
        file.write_all(metadata.to_json().as_bytes())
            .expect("Failed to write to project.json.");
        engine_gui.print_to_terminal("Created project.json with metadata.");
    }

    // Function to check if the project path is valid
    pub fn is_valid_project_path(project_path: &str) -> bool {
        // check if the path is a directory and have project.json
        if Path::new(project_path).exists() && Path::new(project_path).is_dir() && Path::new(&format!("{}/project.json", project_path)).exists() {
            return true;
        }
        return false;
    }

    pub fn list_files_in_folder(folder_path: &str, engine_gui: &mut EngineGui) -> Vec<String> {
        if Path::new(folder_path).exists() {
            match fs::read_dir(folder_path) {
                Ok(read_dir) => {
                    read_dir
                        .filter_map(|entry| entry.ok().map(|e| e.file_name().into_string().unwrap()))
                        .collect()
                }
                Err(_) => {
                    engine_gui.print_to_terminal(&format!("Failed to read folder: {}", folder_path));
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
        fs::read_to_string(file_path).map_err(|err| {
            format!(
                "Failed to load content from file '{}': {}",
                file_path, err
            )
        })
    }
}
