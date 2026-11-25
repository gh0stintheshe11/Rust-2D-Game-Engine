#[cfg(test)]
mod tests {
    use rust_2d_game_engine::project::{ProjectManager, ProjectMetadata};
    use rust_2d_game_engine::scene::SceneManager;
    use std::fs;
    use std::path::PathBuf;

    fn temp_project(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rust2d_pm_test_{}_{}", name, uuid::Uuid::new_v4()));
        fs::create_dir_all(dir.join("scenes")).unwrap();
        fs::create_dir_all(dir.join("assets/images")).unwrap();
        fs::create_dir_all(dir.join("assets/scripts")).unwrap();
        dir
    }

    #[test]
    fn test_scene_paths_saved_relative_loaded_absolute() {
        let project = temp_project("roundtrip");

        // Build a scene whose entity holds absolute paths (in-memory format)
        let mut manager = SceneManager::new();
        let scene_id = manager.create_scene("main").unwrap();
        manager.set_active_scene(scene_id).unwrap();
        let scene = manager.get_scene_mut(scene_id).unwrap();
        let entity_id = scene.create_entity("player").unwrap();
        let entity = scene.get_entity_mut(entity_id).unwrap();
        entity
            .add_image(project.join("assets/images/bird1.png"))
            .unwrap();
        entity
            .set_script(project.join("assets/scripts/script.lua"))
            .unwrap();

        ProjectManager::save_scene_hierarchy(&project, &manager).unwrap();

        // On disk: paths must be relative with forward slashes
        let json = fs::read_to_string(project.join("scenes/scene_manager.json")).unwrap();
        assert!(
            json.contains("\"assets/images/bird1.png\""),
            "image path should be stored relative, got: {}",
            json
        );
        assert!(
            !json.contains(project.to_str().unwrap()),
            "no absolute project prefix may leak into the scene file"
        );

        // On load: paths must be absolute again, under this project root
        let loaded = ProjectManager::load_scene_hierarchy(&project).unwrap();
        let scene = loaded.get_scene(scene_id).unwrap();
        let entity = scene.get_entity(entity_id).unwrap();
        assert_eq!(
            entity.images[0],
            project.join("assets/images/bird1.png"),
            "image path should resolve against the opened project"
        );
        assert_eq!(
            entity.script.as_ref().unwrap(),
            &project.join("assets/scripts/script.lua")
        );
    }

    #[test]
    fn test_legacy_absolute_paths_are_remapped_on_load() {
        let project = temp_project("legacy");

        // Simulate a scene file saved on another machine with absolute paths
        let mut manager = SceneManager::new();
        let scene_id = manager.create_scene("main").unwrap();
        let scene = manager.get_scene_mut(scene_id).unwrap();
        let entity_id = scene.create_entity("bird").unwrap();
        scene
            .get_entity_mut(entity_id)
            .unwrap()
            .add_image(PathBuf::from(
                "/Users/someone_else/old_machine/game/assets/images/bird1.png",
            ))
            .unwrap();

        let json = serde_json::to_string_pretty(&manager).unwrap();
        fs::write(project.join("scenes/scene_manager.json"), json).unwrap();

        let loaded = ProjectManager::load_scene_hierarchy(&project).unwrap();
        let entity = loaded
            .get_scene(scene_id)
            .unwrap()
            .get_entity(entity_id)
            .unwrap();
        assert_eq!(
            entity.images[0],
            project.join("assets/images/bird1.png"),
            "foreign absolute paths should be remapped via their assets/ segment"
        );
    }

    #[test]
    fn test_foreign_absolute_paths_are_remapped_on_any_os() {
        let project = temp_project("foreign");
        let mut manager = SceneManager::new();
        let scene_id = manager.create_scene("main").unwrap();
        let scene = manager.get_scene_mut(scene_id).unwrap();
        let windows = scene.create_entity("windows").unwrap();
        let unix = scene.create_entity("unix").unwrap();
        scene
            .get_entity_mut(windows)
            .unwrap()
            .add_image(PathBuf::from(r"C:\Users\me\game\assets\images\a.png"))
            .unwrap();
        scene
            .get_entity_mut(unix)
            .unwrap()
            .add_image(PathBuf::from("/home/me/game/assets/images/b.png"))
            .unwrap();
        let json = serde_json::to_string_pretty(&manager).unwrap();
        fs::write(project.join("scenes/scene_manager.json"), json).unwrap();

        let loaded = ProjectManager::load_scene_hierarchy(&project).unwrap();
        let scene = loaded.get_scene(scene_id).unwrap();
        assert_eq!(
            scene.get_entity(windows).unwrap().images[0],
            project.join("assets").join("images").join("a.png")
        );
        assert_eq!(
            scene.get_entity(unix).unwrap().images[0],
            project.join("assets").join("images").join("b.png")
        );
    }

    #[test]
    fn test_invalid_numbers_in_old_files_load_as_zero() {
        // Older versions saved NaN/infinity as null, which made the whole
        // scene file unreadable
        let project = temp_project("nulls");
        let mut manager = SceneManager::new();
        let scene_id = manager.create_scene("main").unwrap();
        let entity_id = manager
            .get_scene_mut(scene_id)
            .unwrap()
            .create_entity("e")
            .unwrap();
        let json = serde_json::to_string_pretty(&manager)
            .unwrap()
            .replace("\"Float\": 0.0", "\"Float\": null");
        assert!(json.contains("null"));
        fs::write(project.join("scenes/scene_manager.json"), json).unwrap();

        let loaded = ProjectManager::load_scene_hierarchy(&project).expect("loads");
        let entity = loaded
            .get_scene(scene_id)
            .unwrap()
            .get_entity(entity_id)
            .unwrap();
        assert_eq!(entity.get_x(), 0.0);
    }

    #[test]
    fn test_saves_are_atomic_and_portable() {
        let project = temp_project("atomic");
        let mut manager = SceneManager::new();
        manager.create_scene("main").unwrap();
        let metadata = ProjectMetadata {
            project_name: "atomic".to_string(),
            version: "1.0.0".to_string(),
            project_path: project.to_string_lossy().into_owned(),
        };

        ProjectManager::save_project_full(&project, &metadata, &manager).unwrap();

        let leftovers: Vec<_> = fs::read_dir(project.join("scenes"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .filter(|name| name.to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "no temporary files left: {:?}",
            leftovers
        );
        let epm = fs::read_to_string(project.join("project.epm")).unwrap();
        assert!(
            !epm.contains("project_path"),
            "the project file must not record this machine's path: {}",
            epm
        );
    }

    #[test]
    fn test_projects_without_asset_folders_open() {
        // git doesn't keep empty folders, so a cloned project may lack them
        let project = std::env::temp_dir().join(format!("rust2d_pm_bare_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("project.epm"), r#"{"project_name": "bare"}"#).unwrap();

        let loaded = ProjectManager::load_project_full(&project).expect("opens");
        assert_eq!(loaded.metadata.project_name, "bare");
        assert!(loaded.scene_manager.scenes.is_empty());
    }

    #[test]
    fn test_entity_paths_move_between_projects() {
        use rust_2d_game_engine::scene::Entity;

        let first = temp_project("entity_from");
        let second = temp_project("entity_to");
        let elsewhere = std::env::temp_dir().join("shared").join("theme.ogg");
        let mut entity = Entity::new(uuid::Uuid::new_v4(), "coin").unwrap();
        entity
            .add_image(first.join("assets").join("images").join("coin.png"))
            .unwrap();
        entity.add_sound(elsewhere.clone()).unwrap();
        entity
            .set_script(first.join("assets").join("scripts").join("coin.lua"))
            .unwrap();
        entity.tilemap = Some(Box::new(rust_2d_game_engine::scene::Tilemap::new(
            first.join("assets").join("images").join("tiles.png"),
            (16, 16),
        )));

        ProjectManager::make_entity_paths_relative(&mut entity, &first);
        assert_eq!(
            entity.tilemap.as_ref().unwrap().tileset,
            PathBuf::from("assets/images/tiles.png")
        );
        assert_eq!(entity.images[0], PathBuf::from("assets/images/coin.png"));
        assert_eq!(
            entity.script,
            Some(PathBuf::from("assets/scripts/coin.lua"))
        );
        assert_eq!(entity.sounds[0], elsewhere, "outside the project: kept");

        ProjectManager::resolve_entity_paths(&mut entity, &second);
        assert_eq!(
            entity.images[0],
            second.join("assets").join("images").join("coin.png")
        );
        assert_eq!(
            entity.tilemap.as_ref().unwrap().tileset,
            second.join("assets").join("images").join("tiles.png")
        );
        assert_eq!(
            entity.script,
            Some(second.join("assets").join("scripts").join("coin.lua"))
        );
    }
}
