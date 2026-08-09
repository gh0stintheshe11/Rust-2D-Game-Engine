use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

use strip_ansi_escapes::strip;

use super::ProjectManager;
use crate::logger::LOGGER;

impl ProjectManager {
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
                    {
                        let clean_line = strip(line.as_bytes());
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
                    {
                        let clean_line = strip(line.as_bytes());
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
}
