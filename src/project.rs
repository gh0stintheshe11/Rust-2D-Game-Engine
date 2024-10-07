use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use serde::{Serialize, Deserialize};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::thread;

// Project metadata structure for project.json
#[derive(Serialize, Deserialize, Debug)]
struct ProjectMetadata {
    project_name: String,
    version: String,
    project_path: String,
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
    pub fn create_project(project_name: &str, project_path: &str) {
        let base_path = format!("{}/{}", project_path, project_name);

        // Create main project folder
        FileManagement::create_folder(&base_path);

        // Create subfolders
        FileManagement::create_folder(&format!("{}/assets", base_path));
        FileManagement::create_folder(&format!("{}/assets/images", base_path));
        FileManagement::create_folder(&format!("{}/assets/sounds", base_path));
        FileManagement::create_folder(&format!("{}/assets/fonts", base_path));
        FileManagement::create_folder(&format!("{}/assets/videos", base_path));
        FileManagement::create_folder(&format!("{}/entities", base_path));
        FileManagement::create_folder(&format!("{}/scripts", base_path));
        FileManagement::create_folder(&format!("{}/scenes", base_path));

        // Create project.json
        let metadata = ProjectMetadata {
            project_name: project_name.to_string(),
            version: "1.0.0".to_string(),
            project_path: project_path.to_string(),
        };

        FileManagement::create_project_file(&base_path, &metadata);
        println!("Project '{}' created successfully at {}!", project_name, project_path);
    }

    // Helper function to create folders
    fn create_folder(path: &str) {
        if !Path::new(path).exists() {
            fs::create_dir_all(path).expect("Failed to create folder.");
            println!("Created folder: {}", path);
        }
    }

    // Create project.json file
    fn create_project_file(base_path: &str, metadata: &ProjectMetadata) {
        let file_path = format!("{}/project.json", base_path);
        let mut file = File::create(&file_path).expect("Failed to create project.json.");
        file.write_all(metadata.to_json().as_bytes())
            .expect("Failed to write to project.json.");
        println!("Created project.json with metadata.");
    }

    // Function to check if the project path is valid
    pub fn is_valid_project_path(project_path: &str) -> bool {
        // check if the path is a directory and have project.json
        if Path::new(project_path).exists() && Path::new(project_path).is_dir() && Path::new(&format!("{}/project.json", project_path)).exists() {
            return true;
        }
        return false;
    }

    pub fn list_files_in_folder(folder_path: &str) -> Vec<String> {
        if Path::new(folder_path).exists() {
            match fs::read_dir(folder_path) {
                Ok(read_dir) => {
                    read_dir
                        .filter_map(|entry| entry.ok().map(|e| e.file_name().into_string().unwrap()))
                        .collect()
                }
                Err(_) => {
                    println!("Failed to read folder: {}", folder_path);
                    vec![] // Return an empty vector in case of error
                }
            }
        } else {
            println!("Folder does not exist: {}", folder_path);
            vec![] // Return empty vector if folder does not exist
        }
    }

    // Function to watch for file changes in a folder
    pub fn watch_folder(folder_path: &str) {
        let (tx, rx) = channel();

        // Create a file system watcher
        let mut watcher: RecommendedWatcher = Watcher::new(tx, Config::default()).unwrap();

        // Start watching the folder recursively
        watcher.watch(Path::new(folder_path), RecursiveMode::Recursive).unwrap();

        // Spawn a thread to handle the events
        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(event) => {
                        println!("File system event: {:?}", event);  // Handle the event, like reloading the file list
                    }
                    Err(e) => println!("Watch error: {:?}", e),
                }
            }
        });
    }
}