// Import the ECS module from the main project
use rust_2d_game_engine::ecs::*;

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
        let attr_id = entity.create_attribute(
            "Health",
            AttributeType::Integer,
            AttributeValue::Integer(100)
        ).unwrap();
        
        // Verify attribute
        let attr = entity.get_attribute(attr_id).unwrap();
        assert_eq!(attr.name, "Health");
        assert_eq!(attr.value, AttributeValue::Integer(100));
        
        // Modify attribute
        entity.modify_attribute(
            attr_id,
            None,
            None,
            Some(AttributeValue::Integer(80))
        ).unwrap();
        
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
        let entity_id = scene.create_physical_entity(
            "physical_entity",
            (10.0, 20.0, 30.0),
            physics
        ).unwrap();
        
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
}