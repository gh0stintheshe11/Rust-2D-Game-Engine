use rust_2d_game_engine::scene::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let mut scene = Scene::new("test_scene").unwrap();
        let entity_id = scene.create_entity("test_entity").unwrap();

        let entity = scene.get_entity(entity_id).unwrap();
        assert_eq!(entity.name, "test_entity");

        // Check default position attributes
        assert_eq!(entity.get_x(), 0.0);
        assert_eq!(entity.get_y(), 0.0);
        assert_eq!(entity.get_z(), 0.0);
    }

    #[test]
    fn test_attribute_management() {
        let mut scene = Scene::new("test_scene").unwrap();
        let entity_id = scene.create_entity("test_entity").unwrap();
        let entity = scene.get_entity_mut(entity_id).unwrap();

        // Create attribute
        let attr_id = entity
            .create_attribute(
                "Health",
                AttributeType::Integer,
                AttributeValue::Integer(100),
            )
            .unwrap();

        // Verify attribute
        let attr = entity.get_attribute(attr_id).unwrap();
        assert_eq!(attr.name, "Health");
        assert_eq!(attr.value, AttributeValue::Integer(100));

        // Modify attribute
        entity
            .modify_attribute(attr_id, None, None, Some(AttributeValue::Integer(80)))
            .unwrap();

        // Verify modification
        let attr = entity.get_attribute(attr_id).unwrap();
        assert_eq!(attr.value, AttributeValue::Integer(80));
    }

    #[test]
    fn test_camera_entity() {
        let mut scene = Scene::new("test_scene").unwrap();
        let camera_id = scene.create_camera("main_camera").unwrap();
        let camera = scene.get_entity(camera_id).unwrap();

        assert!(camera.is_camera());
        assert_eq!(camera.get_camera_width(), 800.0);
        assert_eq!(camera.get_camera_height(), 600.0);
        assert_eq!(camera.get_camera_zoom(), 1.0);
        assert_eq!(camera.get_camera_rotation(), 0.0);
    }

    #[test]
    fn test_physical_entity() {
        let mut scene = Scene::new("test_scene").unwrap();
        let physics = PhysicsProperties::default();
        let entity_id = scene
            .create_physical_entity("physical_entity", (10.0, 20.0, 30.0), physics)
            .unwrap();

        let entity = scene.get_entity(entity_id).unwrap();
        assert_eq!(entity.get_position().unwrap(), (10.0, 20.0, 30.0));

        // Verify physics attributes - Fixed version
        if let Ok(attr) = entity.get_attribute_by_name("has_collision") {
            assert_eq!(attr.value, AttributeValue::Boolean(true));
        } else {
            panic!("has_collision attribute not found");
        }
    }

    #[test]
    fn test_scene_management() {
        let mut scene_manager = SceneManager::new();

        // Create scene
        let scene_id = scene_manager.create_scene("test_scene").unwrap();
        assert!(scene_manager.get_scene(scene_id).is_some());

        // Set active scene
        scene_manager.set_active_scene(scene_id).unwrap();
        assert_eq!(scene_manager.active_scene, Some(scene_id));

        // Get active scene
        let active_scene = scene_manager.get_active_scene().unwrap();
        assert_eq!(active_scene.name, "test_scene");
    }

    #[test]
    fn test_shared_entities() {
        let mut scene_manager = SceneManager::new();

        // Create shared entity
        let shared_id = scene_manager.create_shared_entity("shared_entity").unwrap();

        // Create scene and add reference to shared entity
        let scene_id = scene_manager.create_scene("test_scene").unwrap();
        let scene = scene_manager.get_scene_mut(scene_id).unwrap();
        scene.add_shared_entity_ref(shared_id).unwrap();

        // Verify shared entity reference
        assert!(scene.shared_entity_refs.contains(&shared_id));
    }

    #[test]
    fn test_scene_key_is_the_scene_id() {
        let mut manager = SceneManager::new();
        let id = manager.create_scene("level").unwrap();
        assert_eq!(manager.get_scene(id).unwrap().id, id);
    }

    #[test]
    fn test_repair_ids_matches_ids_to_keys() {
        let mut manager = SceneManager::new();
        let id = manager.create_scene("level").unwrap();
        manager.get_scene_mut(id).unwrap().id = uuid::Uuid::new_v4();
        manager.repair_ids();
        assert_eq!(manager.get_scene(id).unwrap().id, id);
    }

    #[test]
    fn test_attribute_values_must_fit_their_type() {
        let mut scene = Scene::new("test_scene").unwrap();
        let entity_id = scene.create_entity("e").unwrap();
        let entity = scene.get_entity_mut(entity_id).unwrap();

        // Values JSON can't represent are refused, not saved as null
        assert!(
            entity
                .create_attribute(
                    "speed",
                    AttributeType::Float,
                    AttributeValue::Float(f32::NAN)
                )
                .is_err()
        );
        let x = entity.get_attribute_by_name("x").unwrap().id;
        assert!(
            entity
                .modify_attribute(x, None, None, Some(AttributeValue::Float(f32::INFINITY)))
                .is_err()
        );
        assert!(
            entity
                .modify_attribute(x, None, None, Some(AttributeValue::Boolean(true)))
                .is_err(),
            "type mismatch"
        );
        assert!(
            entity
                .modify_attribute(x, Some("left".to_string()), None, None)
                .is_err(),
            "core attributes keep their names"
        );
        assert!(
            entity
                .create_attribute("", AttributeType::Integer, AttributeValue::Integer(1))
                .is_err()
        );
        assert_eq!(entity.get_x(), 0.0);
    }

    #[test]
    fn test_camera_attributes_stay_protected_after_rename() {
        let mut scene = Scene::new("test_scene").unwrap();
        let camera_id = scene.default_camera.unwrap();
        let camera = scene.get_entity_mut(camera_id).unwrap();
        camera.change_entity_name("Player View").unwrap();

        let width = camera.get_attribute_by_name("width").unwrap().id;
        assert!(camera.delete_attribute(width).is_err());
    }

    #[test]
    fn test_one_bad_update_does_not_block_the_others() {
        let mut scene = Scene::new("test_scene").unwrap();
        let entity_id = scene.create_entity("e").unwrap();
        let x = scene
            .get_entity(entity_id)
            .unwrap()
            .get_attribute_by_name("x")
            .unwrap()
            .id;

        let result = scene.update_entity_attributes(vec![
            (uuid::Uuid::new_v4(), x, AttributeValue::Float(1.0)),
            (entity_id, x, AttributeValue::Float(42.0)),
        ]);
        assert!(result.is_err(), "the missing entity is reported");
        assert_eq!(scene.get_entity(entity_id).unwrap().get_x(), 42.0);
    }

    #[test]
    fn test_typed_attribute_access() {
        let mut entity = Entity::new(uuid::Uuid::new_v4(), "e").unwrap();
        for (name, value) in [
            ("visible", AttributeValue::Boolean(true)),
            ("lives", AttributeValue::Integer(3)),
            ("label", AttributeValue::String("hi".to_string())),
            ("size", AttributeValue::Vector2(1.0, 2.0)),
        ] {
            entity.set_or_create_attribute(name, value).unwrap();
        }
        assert_eq!(entity.get_bool("visible"), Some(true));
        assert_eq!(
            entity.get_float("lives"),
            Some(3.0),
            "integers read as floats"
        );
        assert_eq!(entity.get_string("label"), Some("hi"));
        assert_eq!(entity.get_vector2("size"), Some((1.0, 2.0)));
        assert_eq!(entity.get_bool("lives"), None, "another type");
        assert_eq!(entity.get_float("missing"), None);

        // Existing attributes are updated in place, and keep their type
        entity
            .set_or_create_attribute("lives", AttributeValue::Integer(2))
            .unwrap();
        assert_eq!(entity.get_float("lives"), Some(2.0));
        assert!(
            entity
                .set_or_create_attribute("lives", AttributeValue::Boolean(false))
                .is_err()
        );
    }
}
