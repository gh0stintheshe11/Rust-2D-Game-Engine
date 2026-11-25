//! Save data: values a game keeps between runs (high scores, progress,
//! settings), stored as one JSON file per game in the user's data folder.
//!
//! See `docs/save.md`.

use crate::logger::LOGGER;
use crate::project::write_atomically;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// Largest save file [`SaveData`] writes, in bytes.
pub const MAX_SAVE_SIZE: usize = 1024 * 1024;

/// Why [`SaveData::set`] failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveError {
    /// The save file would exceed [`MAX_SAVE_SIZE`]; nothing changed.
    TooLarge,
    /// Writing the save file failed; the change is kept in memory.
    Io(String),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::TooLarge => write!(f, "save data can't exceed {} KiB", MAX_SAVE_SIZE / 1024),
            SaveError::Io(message) => f.write_str(message),
        }
    }
}

/// A game's save data: string keys mapped to JSON values, kept in memory
/// and written to the save file after every change.
#[derive(Debug, Default)]
pub struct SaveData {
    // None: in memory only
    path: Option<PathBuf>,
    values: Map<String, Value>,
}

impl SaveData {
    /// Save data that lives only in memory: nothing is read or written.
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// Save data kept in the file at `path`, loaded from it if it exists.
    /// A file that isn't save data (not a JSON object) is renamed to
    /// `<name>.corrupt` and reported, so it isn't overwritten; the game
    /// starts with no saved values.
    pub fn open(path: PathBuf) -> Self {
        let values = match fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<Map<String, Value>>(&json) {
                Ok(values) => values,
                Err(e) => {
                    set_aside(&path, &e.to_string());
                    Map::new()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Map::new(),
            Err(e) => {
                LOGGER.warning(format!("Can't read save data {}: {}", path.display(), e));
                Map::new()
            }
        };
        Self {
            path: Some(path),
            values,
        }
    }

    /// The save data of the game called `game_name`: `save.json` in its
    /// folder of the user's data folder (see [`game_data_dir`]), or in
    /// memory when there's no such folder.
    pub fn for_game(game_name: &str) -> Self {
        match game_data_dir(game_name) {
            Some(dir) => Self::open(dir.join("save.json")),
            None => {
                LOGGER.warning("No user data folder found: save data won't be kept");
                Self::in_memory()
            }
        }
    }

    /// The save file, unless the data lives in memory only.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    /// Set a value, or with `None` remove it, then write the save file. The
    /// change is refused when the file would exceed [`MAX_SAVE_SIZE`]; when
    /// only writing the file fails, it stays in memory.
    pub fn set(&mut self, key: &str, value: Option<Value>) -> Result<(), SaveError> {
        let previous = match value {
            Some(value) => self.values.insert(key.to_string(), value),
            None => self.values.remove(key),
        };
        let json =
            serde_json::to_string_pretty(&self.values).map_err(|e| SaveError::Io(e.to_string()))?;
        if json.len() > MAX_SAVE_SIZE {
            match previous {
                Some(previous) => self.values.insert(key.to_string(), previous),
                None => self.values.remove(key),
            };
            return Err(SaveError::TooLarge);
        }
        self.write(&json).map_err(SaveError::Io)
    }

    /// Remove every value, and the save file.
    pub fn clear(&mut self) -> Result<(), String> {
        self.values.clear();
        match &self.path {
            Some(path) => match fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(format!("Can't delete {}: {}", path.display(), e)),
            },
            None => Ok(()),
        }
    }

    fn write(&self, json: &str) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)
                .map_err(|e| format!("Can't create {}: {}", dir.display(), e))?;
        }
        write_atomically(path, json)
    }
}

/// Rename an unreadable save file out of the way, so saving doesn't destroy
/// what may still be recovered from it.
fn set_aside(path: &Path, problem: &str) {
    let mut corrupt = path.as_os_str().to_os_string();
    corrupt.push(".corrupt");
    let corrupt = PathBuf::from(corrupt);
    match fs::rename(path, &corrupt) {
        Ok(()) => LOGGER.warning(format!(
            "Save data {} isn't valid ({}); moved it to {} and started empty",
            path.display(),
            problem,
            corrupt.display()
        )),
        Err(e) => LOGGER.warning(format!(
            "Save data {} isn't valid ({}) and can't be moved aside: {}",
            path.display(),
            problem,
            e
        )),
    }
}

/// The folder where the game called `game_name` keeps its files: a folder
/// named after the game in the user's data folder, which is `%APPDATA%` on
/// Windows, `~/Library/Application Support` on macOS, and `$XDG_DATA_HOME`
/// or `~/.local/share` elsewhere. Characters that can't be in a folder name
/// become `_`. `None` when the user's data folder is unknown.
pub fn game_data_dir(game_name: &str) -> Option<PathBuf> {
    let absolute = |var: &str| {
        std::env::var_os(var)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    let data_dir = if cfg!(windows) {
        absolute("APPDATA")
    } else if cfg!(target_os = "macos") {
        absolute("HOME").map(|home| home.join("Library").join("Application Support"))
    } else {
        absolute("XDG_DATA_HOME")
            .or_else(|| absolute("HOME").map(|home| home.join(".local").join("share")))
    }?;
    Some(data_dir.join(folder_name(game_name)))
}

/// A folder name for a game: letters, digits, spaces, `-`, `_` and `.` are
/// kept, anything else becomes `_`; never empty, `.` or `..`.
fn folder_name(game_name: &str) -> String {
    let name: String = game_name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let name = name.trim_matches(|c| c == '.' || c == ' ');
    if name.is_empty() {
        "game".to_string()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::folder_name;

    #[test]
    fn test_folder_names() {
        assert_eq!(folder_name("Star Catcher"), "Star Catcher");
        assert_eq!(folder_name("My Game: Part 2/3"), "My Game_ Part 2_3");
        assert_eq!(folder_name(" .. "), "game");
        assert_eq!(folder_name(""), "game");
        assert_eq!(folder_name("星を集めて"), "星を集めて");
    }
}
