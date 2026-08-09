#[cfg(test)]
mod tests {
    use rust_2d_game_engine::ecs::SceneManager;
    use rust_2d_game_engine::project_manager::ProjectManager;
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
}
