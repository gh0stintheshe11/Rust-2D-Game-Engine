//! Save data: kept in a JSON file between runs, robust against bad files.

use rust_2d_game_engine::save::{MAX_SAVE_SIZE, SaveData, SaveError, game_data_dir};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

/// A save file path in a fresh folder that doesn't exist yet.
fn save_file(test: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!("rust2d_save_{}_{}", test, uuid::Uuid::new_v4()))
        .join("save.json")
}

#[test]
fn test_values_are_kept_between_runs() {
    let path = save_file("between_runs");
    let mut save = SaveData::open(path.clone());
    assert_eq!(save.get("best"), None, "no file yet");
    save.set("best", Some(json!(12))).unwrap();
    save.set("name", Some(json!("ada"))).unwrap();
    save.set("levels", Some(json!({"forest": true, "times": [3.5, 4]})))
        .unwrap();
    assert!(path.is_file(), "written right away, folders included");

    let mut again = SaveData::open(path.clone());
    assert_eq!(again.get("best"), Some(&json!(12)));
    assert_eq!(
        again.get("levels"),
        Some(&json!({"forest": true, "times": [3.5, 4]}))
    );

    again.set("name", None).unwrap();
    assert_eq!(SaveData::open(path.clone()).get("name"), None, "removed");

    again.clear().unwrap();
    assert!(!path.exists());
    assert_eq!(again.get("best"), None);
    again.clear().unwrap();
}

#[test]
fn test_a_corrupt_file_is_set_aside_not_overwritten() {
    let path = save_file("corrupt");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "{ not json").unwrap();

    let mut save = SaveData::open(path.clone());
    assert_eq!(save.get("best"), None);
    let corrupt = path.with_file_name("save.json.corrupt");
    assert_eq!(fs::read_to_string(&corrupt).unwrap(), "{ not json");
    save.set("best", Some(json!(1))).unwrap();
    assert_eq!(SaveData::open(path).get("best"), Some(&json!(1)));
}

#[test]
fn test_save_data_has_a_size_limit() {
    let path = save_file("limit");
    let mut save = SaveData::open(path.clone());
    save.set("small", Some(json!("x"))).unwrap();
    let huge = "x".repeat(MAX_SAVE_SIZE);
    assert_eq!(
        save.set("huge", Some(json!(huge))),
        Err(SaveError::TooLarge)
    );
    assert_eq!(save.get("huge"), None, "nothing changed");
    assert_eq!(SaveData::open(path).get("small"), Some(&json!("x")));
}

#[test]
fn test_in_memory_save_data_writes_nothing() {
    let mut save = SaveData::in_memory();
    assert_eq!(save.path(), None);
    save.set("best", Some(json!(3))).unwrap();
    assert_eq!(save.get("best"), Some(&json!(3)));
    save.clear().unwrap();
}

#[test]
fn test_game_folders_are_named_after_the_game() {
    // CI machines and desktops have a home or application data folder
    let dir = game_data_dir("Star Catcher: Deluxe").expect("a user data folder");
    assert!(dir.ends_with("Star Catcher_ Deluxe"), "{}", dir.display());
    assert!(dir.is_absolute());
}
