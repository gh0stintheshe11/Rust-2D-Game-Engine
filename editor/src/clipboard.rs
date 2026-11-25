//! Entities on the system clipboard, as JSON text that this or another
//! editor window can paste, into this project or another one.

use rust_2d_game_engine::project::ProjectManager;
use rust_2d_game_engine::scene::Entity;
use std::path::Path;

/// Marks clipboard text as an entity copied by the editor.
const FORMAT: &str = "rust-2d-game-engine/entity";

/// Clipboard text for `entity`. Its resource paths are stored relative to
/// the project, as in scene files, so pasting into another project finds
/// that project's assets with the same paths.
pub fn to_clipboard_text(entity: &Entity, project_path: &Path) -> String {
    let mut portable = entity.clone();
    ProjectManager::make_entity_paths_relative(&mut portable, project_path);
    serde_json::json!({ "format": FORMAT, "entity": portable }).to_string()
}

/// The entity in text written by [`to_clipboard_text`], with its paths
/// resolved against `project_path`; `None` for any other text.
pub fn from_clipboard_text(text: &str, project_path: &Path) -> Option<Entity> {
    let mut value: serde_json::Value = serde_json::from_str(text).ok()?;
    if value.get("format")?.as_str()? != FORMAT {
        return None;
    }
    let mut entity: Entity = serde_json::from_value(value.get_mut("entity")?.take()).ok()?;
    ProjectManager::resolve_entity_paths(&mut entity, project_path);
    Some(entity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_entities_survive_the_clipboard() {
        let first = PathBuf::from("/projects/first");
        let second = PathBuf::from("/projects/second");
        let mut entity = Entity::new(uuid::Uuid::new_v4(), "coin").unwrap();
        entity
            .add_image(first.join("assets").join("images").join("coin.png"))
            .unwrap();

        let text = to_clipboard_text(&entity, &first);
        assert!(
            text.contains("assets/images/coin.png") && !text.contains("first"),
            "paths are project-relative: {}",
            text
        );
        let pasted = from_clipboard_text(&text, &second).expect("an entity");
        assert_eq!(pasted.name, "coin");
        assert_eq!(pasted.attributes, entity.attributes);
        assert_eq!(
            pasted.images,
            vec![second.join("assets").join("images").join("coin.png")]
        );

        for other in [
            "",
            "coin",
            "{}",
            r#"{"format": "something else", "entity": {}}"#,
        ] {
            assert!(from_clipboard_text(other, &second).is_none(), "{:?}", other);
        }
    }
}
