use std::fs;
use std::path::Path;

use super::ProjectManager;

impl ProjectManager {
    pub fn import_asset(
        project_path: &Path,
        asset_path: &Path,
        asset_type: AssetType,
    ) -> Result<String, String> {
        // Validate file extension
        let extension = asset_path
            .extension()
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
        let file_name = asset_path
            .file_name()
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
        fs::copy(asset_path, &target_path).map_err(|e| format!("Failed to copy asset: {}", e))?;

        // Return relative path from project root
        Ok(target_path
            .strip_prefix(project_path)
            .map_err(|e| format!("Failed to get relative path: {}", e))?
            .to_string_lossy()
            .into_owned())
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
            AssetType::Script => &["lua"], // Currently only supporting Lua scripts
        }
    }
}
