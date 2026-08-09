#[cfg(test)]
mod tests {
    use rust_2d_game_engine::ecs::{PhysicsProperties, Scene};
    use rust_2d_game_engine::physics_engine::PhysicsEngine;

    #[test]
    fn test_initialization() {
        let physics_engine = PhysicsEngine::new();
        assert_eq!(physics_engine.get_time_step(), 1.0 / 60.0);
        assert!(physics_engine.is_empty());
    }

    #[test]
    fn test_physical_entity_creation() {
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics_engine = PhysicsEngine::new();

        // Create a physical entity
        let physics_props = PhysicsProperties {
            is_movable: true,
            affected_by_gravity: true,
            has_collision: true,
            ..Default::default()
        };

        let entity_id = scene
            .create_physical_entity("test_entity", (0.0, 10.0, 0.0), physics_props)
            .unwrap();

        // Add entity to physics engine
        let entity = scene.get_entity(entity_id).unwrap();
        physics_engine.add_entity(entity);

        assert!(physics_engine.has_rigid_body(&entity_id));
        assert!(physics_engine.has_collider(&entity_id));
    }

    #[test]
    fn test_gravity_simulation() {
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics_engine = PhysicsEngine::new();

        // Create a physical entity affected by gravity
        let physics_props = PhysicsProperties {
            is_movable: true,
            affected_by_gravity: true,
            has_collision: true,
            ..Default::default()
        };

        let entity_id = scene
            .create_physical_entity("falling_entity", (0.0, 10.0, 0.0), physics_props)
            .unwrap();

        // Get initial position
        let initial_y = scene.get_entity(entity_id).unwrap().get_y();
        assert_eq!(initial_y, 10.0, "Entity should spawn at the requested y");

        // Add entity to physics engine
        physics_engine.add_entity(scene.get_entity(entity_id).unwrap());

        // Run simulation for more steps to ensure visible movement
        for _ in 0..120 {
            // Increased from 60 to 120 steps
            let updates = physics_engine.step(&mut scene);
            scene.update_entity_attributes(updates).unwrap();
        }

        // Get final position and check with a reasonable threshold.
        // The engine uses screen-space coordinates: +Y points down, so a
        // falling entity's y must increase.
        let final_y = scene.get_entity(entity_id).unwrap().get_y();
        assert!(
            final_y > initial_y + 1.0, // Ensure significant movement
            "Entity should have fallen due to gravity (+Y is down). Initial Y: {}, Final Y: {}",
            initial_y,
            final_y
        );
    }

    #[test]
    fn test_collision_detection() {
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics_engine = PhysicsEngine::new();

        // Create ground below the falling object (screen space: +Y is down)
        let ground_props = PhysicsProperties {
            is_movable: false,
            affected_by_gravity: false,
            has_collision: true,
            ..Default::default()
        };

        let ground_id = scene
            .create_physical_entity("ground", (0.0, 10.0, 0.0), ground_props)
            .unwrap();

        // Create falling object above the ground
        let falling_props = PhysicsProperties {
            is_movable: true,
            affected_by_gravity: true,
            has_collision: true,
            ..Default::default()
        };

        let falling_id = scene
            .create_physical_entity("falling_object", (0.0, 0.0, 0.0), falling_props)
            .unwrap();

        // Add entities to physics engine
        physics_engine.add_entity(scene.get_entity(ground_id).unwrap());
        physics_engine.add_entity(scene.get_entity(falling_id).unwrap());

        // Run simulation and check for collisions
        let mut collision_detected = false;
        for _ in 0..60 {
            physics_engine.step(&mut scene);
            let colliding = physics_engine.get_colliding_entities(&falling_id);
            if colliding.contains(&ground_id) {
                collision_detected = true;
                break;
            }
        }

        assert!(collision_detected, "Collision should have been detected");
    }

    #[test]
    fn test_cleanup() {
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics_engine = PhysicsEngine::new();

        // Create and add entity
        let physics_props = PhysicsProperties::default();
        let entity_id = scene
            .create_physical_entity("test_entity", (0.0, 0.0, 0.0), physics_props)
            .unwrap();

        physics_engine.add_entity(scene.get_entity(entity_id).unwrap());

        // Verify entity is added
        assert!(!physics_engine.is_empty());

        // Cleanup
        physics_engine.cleanup();

        // Verify everything is cleared
        assert!(physics_engine.is_empty());
    }

    #[test]
    fn test_re_adding_entity_does_not_leak_bodies() {
        // Regression test: re-loading a scene (e.g. play -> pause -> resume)
        // must not create duplicate rigid bodies for the same entity.
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics_engine = PhysicsEngine::new();

        let physics_props = PhysicsProperties {
            is_movable: true,
            affected_by_gravity: true,
            has_collision: true,
            ..Default::default()
        };

        let entity_id = scene
            .create_physical_entity("test_entity", (3.0, 4.0, 0.0), physics_props)
            .unwrap();

        physics_engine.load_scene(&scene);
        assert_eq!(physics_engine.rigid_body_count(), 1);

        // Re-load the same scene twice more
        physics_engine.load_scene(&scene);
        physics_engine.load_scene(&scene);

        assert_eq!(
            physics_engine.rigid_body_count(),
            1,
            "Re-loading a scene must replace bodies, not duplicate them"
        );
        assert!(physics_engine.has_rigid_body(&entity_id));
    }

    #[test]
    fn test_spawn_position_uses_xy_attributes() {
        // Regression test: entities created via create_physical_entity store
        // x/y float attributes; the physics engine must spawn bodies there
        // instead of defaulting to the origin.
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics_engine = PhysicsEngine::new();

        let physics_props = PhysicsProperties {
            is_movable: false, // fixed body: position must come from spawn, not simulation
            affected_by_gravity: false,
            has_collision: true,
            ..Default::default()
        };

        let entity_id = scene
            .create_physical_entity("anchored_entity", (25.0, -7.5, 0.0), physics_props)
            .unwrap();

        physics_engine.add_entity(scene.get_entity(entity_id).unwrap());

        // Step once and apply updates; x/y must reflect the spawn position
        let updates = physics_engine.step(&mut scene);
        scene.update_entity_attributes(updates).unwrap();

        let entity = scene.get_entity(entity_id).unwrap();
        assert_eq!(entity.get_x(), 25.0, "Body should spawn at the entity's x");
        assert_eq!(entity.get_y(), -7.5, "Body should spawn at the entity's y");
    }

    #[test]
    fn test_gravity_scale_attribute() {
        // Two identical falling entities; one with gravity_scale = 3 must
        // fall noticeably farther in the same time.
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics_engine = PhysicsEngine::new();

        let props = PhysicsProperties {
            is_movable: true,
            affected_by_gravity: true,
            // Colliders give the bodies mass; without mass gravity can't act
            has_collision: true,
            ..Default::default()
        };

        let normal_id = scene
            .create_physical_entity("normal", (0.0, 0.0, 0.0), props.clone())
            .unwrap();
        let heavy_id = scene
            .create_physical_entity("heavy", (100.0, 0.0, 0.0), props)
            .unwrap();
        scene
            .get_entity_mut(heavy_id)
            .unwrap()
            .create_attribute(
                "gravity_scale",
                rust_2d_game_engine::ecs::AttributeType::Float,
                rust_2d_game_engine::ecs::AttributeValue::Float(3.0),
            )
            .unwrap();

        physics_engine.add_entity(scene.get_entity(normal_id).unwrap());
        physics_engine.add_entity(scene.get_entity(heavy_id).unwrap());

        for _ in 0..60 {
            let updates = physics_engine.step(&mut scene);
            scene.update_entity_attributes(updates).unwrap();
        }

        let normal_y = scene.get_entity(normal_id).unwrap().get_y();
        let heavy_y = scene.get_entity(heavy_id).unwrap().get_y();
        assert!(
            heavy_y > normal_y * 2.0,
            "gravity_scale=3 should fall much faster: normal_y={}, heavy_y={}",
            normal_y,
            heavy_y
        );
    }
}
