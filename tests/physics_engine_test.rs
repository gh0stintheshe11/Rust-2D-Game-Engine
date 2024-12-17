#[cfg(test)]
mod tests {
    use rust_2d_game_engine::physics_engine::PhysicsEngine;
    use rust_2d_game_engine::ecs::{Scene, Entity, PhysicsProperties};
    use rapier2d::prelude::*;

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

        let entity_id = scene.create_physical_entity(
            "test_entity",
            (0.0, 10.0, 0.0),
            physics_props
        ).unwrap();

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

        let entity_id = scene.create_physical_entity(
            "falling_entity",
            (0.0, 10.0, 0.0),
            physics_props
        ).unwrap();

        // Get initial position
        let initial_y = scene.get_entity(entity_id).unwrap().get_y();

        // Add entity to physics engine
        physics_engine.add_entity(scene.get_entity(entity_id).unwrap());

        // Run simulation for more steps to ensure visible movement
        for _ in 0..120 { // Increased from 60 to 120 steps
            let updates = physics_engine.step(&mut scene);
            scene.update_entity_attributes(updates).unwrap();
        }

        // Get final position and check with a reasonable threshold
        let final_y = scene.get_entity(entity_id).unwrap().get_y();
        assert!(
            final_y < initial_y - 1.0, // Ensure significant movement
            "Entity should have fallen due to gravity. Initial Y: {}, Final Y: {}",
            initial_y,
            final_y
        );
    }

    #[test]
    fn test_collision_detection() {
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics_engine = PhysicsEngine::new();

        // Create ground
        let ground_props = PhysicsProperties {
            is_movable: false,
            affected_by_gravity: false,
            has_collision: true,
            ..Default::default()
        };

        let ground_id = scene.create_physical_entity(
            "ground",
            (0.0, 0.0, 0.0),
            ground_props
        ).unwrap();

        // Create falling object
        let falling_props = PhysicsProperties {
            is_movable: true,
            affected_by_gravity: true,
            has_collision: true,
            ..Default::default()
        };

        let falling_id = scene.create_physical_entity(
            "falling_object",
            (0.0, 5.0, 0.0),
            falling_props
        ).unwrap();

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
        let entity_id = scene.create_physical_entity(
            "test_entity",
            (0.0, 0.0, 0.0),
            physics_props
        ).unwrap();
        
        physics_engine.add_entity(scene.get_entity(entity_id).unwrap());
        
        // Verify entity is added
        assert!(!physics_engine.is_empty());
        
        // Cleanup
        physics_engine.cleanup();
        
        // Verify everything is cleared
        assert!(physics_engine.is_empty());
    }
}
