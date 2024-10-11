#[cfg(test)]
mod tests {
    use rust_2d_game_engine::physics_engine::PhysicsEngine;
    use rapier2d::prelude::*;

    #[test]
    fn test_initialization() {
        let physics_engine = PhysicsEngine::new();

        // Verify gravity is set correctly
        assert_eq!(physics_engine.gravity.x, 0.0);
        assert_eq!(physics_engine.gravity.y, -9.81);

        // Verify that the rigid body and collider sets are empty
        assert_eq!(physics_engine.rigid_body_set.len(), 0);
        assert_eq!(physics_engine.collider_set.len(), 0);
    }

    #[test]
    fn test_add_dynamic_rigid_body_with_collider() {
        let mut physics_engine = PhysicsEngine::new();

        // Add a dynamic rigid body
        let (rb_handle, _collider_handle) = physics_engine
            .add_rigid_body([1.0, 2.0], true)
            .expect("Failed to add dynamic rigid body");

        // Verify that the rigid body and collider are added
        assert_eq!(physics_engine.rigid_body_set.len(), 1);
        assert_eq!(physics_engine.collider_set.len(), 1);

        // Retrieve the rigid body and verify its properties
        let body = physics_engine
            .rigid_body_set
            .get(rb_handle)
            .expect("Failed to retrieve rigid body");
        assert!(body.is_dynamic());
        assert_eq!(body.translation().x, 1.0);
        assert_eq!(body.translation().y, 2.0);
    }

    #[test]
    fn test_add_static_rigid_body_with_collider() {
        let mut physics_engine = PhysicsEngine::new();

        // Add a static rigid body
        let (rb_handle, _collider_handle) = physics_engine
            .add_rigid_body([3.0, 4.0], false)
            .expect("Failed to add static rigid body");

        // Verify that the rigid body and collider are added
        assert_eq!(physics_engine.rigid_body_set.len(), 1);
        assert_eq!(physics_engine.collider_set.len(), 1);

        // Retrieve the rigid body and verify its properties
        let body = physics_engine
            .rigid_body_set
            .get(rb_handle)
            .expect("Failed to retrieve rigid body");
        assert!(body.is_fixed());
        assert_eq!(body.translation().x, 3.0);
        assert_eq!(body.translation().y, 4.0);
    }

    #[test]
    fn test_simulation_under_gravity() {
        let mut physics_engine = PhysicsEngine::new();

        // Add a dynamic body at a non-zero height
        let (handle, _collider_handle) = physics_engine
            .add_rigid_body([0.0, 10.0], true)
            .expect("Failed to add dynamic rigid body");

        // Run the simulation for multiple steps
        for _ in 0..50 {
            physics_engine.step();
        }

        // Retrieve the body and verify it has fallen
        let body = physics_engine
            .rigid_body_set
            .get(handle)
            .expect("Failed to retrieve rigid body");
        assert!(
            body.translation().y < 10.0,
            "Body did not fall under gravity."
        );
    }

    #[test]
    fn test_collision_detection() {
        let mut physics_engine = PhysicsEngine::new();

        // Add a static ground body
        let (_ground_rb_handle, ground_collider_handle) = physics_engine
            .add_rigid_body([0.0, 0.0], false)
            .expect("Failed to add ground body");

        // Add a dynamic body above the ground
        let (_dynamic_rb_handle, dynamic_collider_handle) = physics_engine
            .add_rigid_body([0.0, 2.0], true)
            .expect("Failed to add dynamic body");

        // Run the simulation to allow the dynamic body to fall onto the ground
        for _ in 0..100 {
            physics_engine.step();
        }

        // Check for collisions
        let mut collision_detected = false;
        for contact_pair in physics_engine.narrow_phase.contact_pairs() {
            if (contact_pair.collider1 == ground_collider_handle || contact_pair.collider2 == ground_collider_handle)
                && (contact_pair.collider1 == dynamic_collider_handle || contact_pair.collider2 == dynamic_collider_handle)
            {
                collision_detected = true;
                break;
            }
        }

        assert!(
            collision_detected,
            "No collision detected between dynamic body and ground."
        );
    }

    #[test]
    fn test_add_invalid_rigid_body() {
        let mut physics_engine = PhysicsEngine::new();

        // Attempt to add a body with invalid position
        let result = physics_engine.add_rigid_body([f32::NAN, f32::NAN], true);

        // Verify that the body was not added
        assert!(result.is_none());
        assert_eq!(physics_engine.rigid_body_set.len(), 0);
    }

    #[test]
    fn test_multiple_bodies_falling() {
        let mut physics_engine = PhysicsEngine::new();

        // Add multiple dynamic bodies at different heights
        let handles: Vec<_> = (0..5)
            .map(|i| {
                let (rb_handle, _collider_handle) = physics_engine
                    .add_rigid_body([0.0, 5.0 + i as f32 * 5.0], true)
                    .expect("Failed to add dynamic rigid body");
                rb_handle
            })
            .collect();

        // Run the simulation
        for _ in 0..200 {
            physics_engine.step();
        }

        // Verify that all bodies have fallen
        for handle in handles {
            let body = physics_engine
                .rigid_body_set
                .get(handle)
                .expect("Failed to retrieve rigid body");
            assert!(
                body.translation().y < 5.0,
                "Body did not fall under gravity."
            );
        }
    }

    #[test]
    fn test_different_collider_shapes() {
        let mut physics_engine = PhysicsEngine::new();

        // Function to add a body with a custom collider shape
        fn add_body_with_shape(
            engine: &mut PhysicsEngine,
            position: [f32; 2],
            shape: Collider,
        ) -> RigidBodyHandle {
            let rigid_body = RigidBodyBuilder::dynamic()
                .translation(vector![position[0], position[1]])
                .build();
            let rb_handle = engine.rigid_body_set.insert(rigid_body);
            engine
                .collider_set
                .insert_with_parent(shape, rb_handle, &mut engine.rigid_body_set);
            rb_handle
        }

        // Add bodies with different shapes
        let shapes = vec![
            ColliderBuilder::ball(0.5).build(),
            ColliderBuilder::cuboid(0.5, 0.5).build(),
            ColliderBuilder::capsule_y(0.5, 0.5).build(),
        ];

        let handles: Vec<_> = shapes
            .into_iter()
            .enumerate()
            .map(|(i, shape)| {
                add_body_with_shape(
                    &mut physics_engine,
                    [i as f32 * 2.0, 10.0],
                    shape,
                )
            })
            .collect();

        // Run the simulation
        for _ in 0..100 {
            physics_engine.step();
        }

        // Verify that all bodies have fallen
        for handle in handles {
            let body = physics_engine
                .rigid_body_set
                .get(handle)
                .expect("Failed to retrieve rigid body");
            assert!(
                body.translation().y < 10.0,
                "Body did not fall under gravity."
            );
        }
    }

    #[test]
    fn test_rigid_body_with_custom_properties() {
        let mut physics_engine = PhysicsEngine::new();

        // Add a dynamic body with custom mass and friction
        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 10.0])
            .additional_mass(2.0)
            .build();
        let rb_handle = physics_engine.rigid_body_set.insert(rigid_body);

        // Attach a collider with custom restitution (bounciness)
        let collider = ColliderBuilder::ball(0.5)
            .restitution(0.9)
            .build();
        physics_engine
            .collider_set
            .insert_with_parent(collider, rb_handle, &mut physics_engine.rigid_body_set);

        // Add a static ground
        let _ground_handle = physics_engine
            .add_rigid_body([0.0, 0.0], false)
            .expect("Failed to add ground body");

        // Run the simulation
        for _ in 0..200 {
            physics_engine.step();
        }

        // Retrieve the body and verify it has bounced
        let body = physics_engine
            .rigid_body_set
            .get(rb_handle)
            .expect("Failed to retrieve rigid body");

        assert!(
            body.translation().y > 0.0,
            "Body did not bounce after collision."
        );
    }

    #[test]
    fn test_collision_events() {
        let mut physics_engine = PhysicsEngine::new();

        // Add a static ground body
        let (_ground_rb_handle, ground_collider_handle) = physics_engine
            .add_rigid_body([0.0, 0.0], false)
            .expect("Failed to add ground body");

        // Add a dynamic body above the ground
        let (_dynamic_rb_handle, dynamic_collider_handle) = physics_engine
            .add_rigid_body([0.0, 2.0], true)
            .expect("Failed to add dynamic body");

        // Simulate physics and collect collision events
        let mut collision_pairs = Vec::new();
        for _ in 0..100 {
            physics_engine.step();

            for contact_pair in physics_engine.narrow_phase.contact_pairs() {
                collision_pairs.push((
                    contact_pair.collider1,
                    contact_pair.collider2,
                ));
            }
        }

        // Check that the collision between the two bodies occurred
        let collision_occurred = collision_pairs.iter().any(|&(c1, c2)| {
            (c1 == ground_collider_handle && c2 == dynamic_collider_handle)
                || (c1 == dynamic_collider_handle && c2 == ground_collider_handle)
        });

        assert!(
            collision_occurred,
            "Expected collision between dynamic body and ground did not occur."
        );
    }
}
