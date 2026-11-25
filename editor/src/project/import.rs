use std::fs;
use std::path::Path;

/// Copy `asset_path` into the project's `assets/<type>/` folder and return
/// its project-relative path.
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
        AssetType::Script => project_path.join("assets/scripts"),
    };

    // Get filename and create target path
    let file_name = asset_path
        .file_name()
        .ok_or("Invalid asset path")?
        .to_str()
        .ok_or("Invalid asset filename")?;

    let target_path = target_dir.join(file_name);
    // The folder may be missing, e.g. in a project cloned from git
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create {}: {}", target_dir.display(), e))?;

    // Check for duplicate files
    if target_path.exists() {
        return Err(format!(
            "Asset '{}' already exists in the project. Please rename the file or remove the existing one.",
            file_name
        ));
    }

    // Copy the asset file
    fs::copy(asset_path, &target_path).map_err(|e| format!("Failed to copy asset: {}", e))?;

    // Return the project-relative path, with `/` separators on every OS
    let relative = target_path
        .strip_prefix(project_path)
        .map_err(|e| format!("Failed to get relative path: {}", e))?;
    Ok(relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

/// Kinds of files the editor imports, each into its own `assets/` folder.
#[derive(Debug, Clone, Copy)]
pub enum AssetType {
    Image,  // Image files (textures, sprites)
    Sound,  // Audio files
    Script, // Script files (Lua)
}

impl AssetType {
    /// Accepted file extensions (lowercase).
    pub fn valid_extensions(&self) -> &[&str] {
        match self {
            AssetType::Image => &["png", "jpg", "jpeg", "gif"],
            AssetType::Sound => &["wav", "mp3", "ogg", "flac"],
            AssetType::Script => &["lua"], // Currently only supporting Lua scripts
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sounds_import_in_every_playable_format() {
        let guard = tempfile::Builder::new()
            .prefix("rust2d_import_")
            .tempdir()
            .unwrap();
        let dir = guard.path();
        let project = dir.join("project");
        fs::create_dir_all(&project).unwrap();
        // Distinct base names: Windows and macOS file names are
        // case-insensitive, so "a.flac" and "a.FLAC" would be the same file
        for (index, extension) in ["wav", "mp3", "ogg", "flac", "FLAC"].iter().enumerate() {
            let file = dir.join(format!("sound_{}.{}", index, extension));
            fs::write(&file, b"not decoded by the import").unwrap();
            let imported = import_asset(&project, &file, AssetType::Sound)
                .unwrap_or_else(|e| panic!(".{} refused: {}", extension, e));
            assert_eq!(
                imported,
                format!("assets/sounds/sound_{}.{}", index, extension)
            );
        }

        let text = dir.join("notes.txt");
        fs::write(&text, b"").unwrap();
        assert!(import_asset(&project, &text, AssetType::Sound).is_err());
    }
}
