#[cfg(test)]
mod tests {
    use rust_2d_game_engine::physics::{
        ColliderShape, DEFAULT_GRAVITY, DEFAULT_TIME_STEP, PhysicsEngine,
    };
    use rust_2d_game_engine::scene::{
        AttributeType, AttributeValue, Entity, PhysicsProperties, Scene,
    };
    use uuid::Uuid;

    /// Physical entity with an explicit collider size (no image needed).
    fn sized_entity(
        scene: &mut Scene,
        name: &str,
        top_left: (f32, f32),
        size: (f32, f32),
        props: PhysicsProperties,
    ) -> Uuid {
        let id = scene
            .create_physical_entity(name, (top_left.0, top_left.1, 0.0), props)
            .unwrap();
        let entity = scene.get_entity_mut(id).unwrap();
        set_float(entity, "collider_width", size.0);
        set_float(entity, "collider_height", size.1);
        id
    }

    fn set_float(entity: &mut Entity, name: &str, value: f32) {
        entity
            .create_attribute(name, AttributeType::Float, AttributeValue::Float(value))
            .unwrap();
    }

    fn float_attr(scene: &Scene, id: Uuid, name: &str) -> Option<f32> {
        match scene.get_entity(id).unwrap().get_attribute_by_name(name) {
            Ok(attr) => match attr.value {
                AttributeValue::Float(v) => Some(v),
                _ => None,
            },
            Err(_) => None,
        }
    }

    fn dynamic(gravity: bool) -> PhysicsProperties {
        PhysicsProperties {
            is_movable: true,
            affected_by_gravity: gravity,
            has_collision: true,
            ..Default::default()
        }
    }

    fn fixed() -> PhysicsProperties {
        PhysicsProperties::default()
    }

    /// Run `seconds` of game time the way the runtime does: one script
    /// "frame" of `frame_dt` (here: apply `force` to `id`), then as many
    /// fixed physics steps as the accumulated time allows.
    fn simulate_frames(
        physics: &mut PhysicsEngine,
        scene: &mut Scene,
        frame_dt: f32,
        seconds: f32,
        mut each_frame: impl FnMut(&mut PhysicsEngine),
    ) {
        let mut accumulator = 0.0;
        let mut elapsed = 0.0;
        while elapsed < seconds - 1e-6 {
            physics.begin_frame(frame_dt);
            each_frame(physics);
            accumulator += frame_dt;
            while accumulator >= DEFAULT_TIME_STEP - 1e-6 {
                accumulator -= DEFAULT_TIME_STEP;
                physics.step(scene);
            }
            elapsed += frame_dt;
        }
    }

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

        assert!(physics_engine.has_rigid_body(entity_id));
        assert!(physics_engine.has_collider(entity_id));
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
            physics_engine.step(&mut scene);
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
            let colliding = physics_engine.get_colliding_entities(falling_id);
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
        assert!(physics_engine.has_rigid_body(entity_id));
    }

    #[test]
    fn test_spawn_position_uses_xy_attributes() {
        // Regression test: entities created via create_physical_entity store
        // x/y float attributes; the physics engine must spawn bodies there
        // instead of defaulting to the origin.
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics_engine = PhysicsEngine::new();

        let physics_props = PhysicsProperties {
            // dynamic without gravity: the step writes the body position back
            is_movable: true,
            affected_by_gravity: false,
            has_collision: true,
            ..Default::default()
        };

        let entity_id = scene
            .create_physical_entity("anchored_entity", (25.0, -7.5, 0.0), physics_props)
            .unwrap();

        physics_engine.add_entity(scene.get_entity(entity_id).unwrap());

        // Step once; x/y must reflect the spawn position
        physics_engine.step(&mut scene);

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
                rust_2d_game_engine::scene::AttributeType::Float,
                rust_2d_game_engine::scene::AttributeValue::Float(3.0),
            )
            .unwrap();

        physics_engine.add_entity(scene.get_entity(normal_id).unwrap());
        physics_engine.add_entity(scene.get_entity(heavy_id).unwrap());

        for _ in 0..60 {
            physics_engine.step(&mut scene);
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

    #[test]
    fn test_resting_bodies_stay_simulated() {
        // A stack resting on the floor must settle, keep its contacts, and
        // still respond to scripts afterwards.
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics = PhysicsEngine::new();
        let floor = sized_entity(&mut scene, "floor", (0.0, 100.0), (400.0, 20.0), fixed());
        let lower = sized_entity(
            &mut scene,
            "lower",
            (90.0, 75.0),
            (20.0, 20.0),
            dynamic(true),
        );
        let upper = sized_entity(
            &mut scene,
            "upper",
            (90.0, 50.0),
            (20.0, 20.0),
            dynamic(true),
        );
        physics.load_scene(&scene);

        for _ in 0..300 {
            physics.step(&mut scene);
        }
        let lower_y = scene.get_entity(lower).unwrap().get_y();
        let upper_y = scene.get_entity(upper).unwrap().get_y();
        assert!(
            (lower_y - 80.0).abs() < 0.5,
            "lower box rests on the floor: {}",
            lower_y
        );
        assert!(
            (upper_y - 60.0).abs() < 0.5,
            "upper box rests on the lower: {}",
            upper_y
        );
        assert!(physics.get_colliding_entities(lower).contains(&floor));

        physics.set_velocity(lower, (100.0, 0.0));
        for _ in 0..10 {
            physics.step(&mut scene);
        }
        assert!(
            scene.get_entity(lower).unwrap().get_x() > 95.0,
            "a resting body must still respond to set_velocity"
        );
    }

    #[test]
    fn test_rotation_is_written_back_around_the_center() {
        // A spinning body reports its angle through the `rotation`
        // attribute; it rotates around the sprite center, so the top-left
        // corner of the unrotated sprite (x/y) stays put.
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics = PhysicsEngine::new();
        let props = PhysicsProperties {
            can_rotate: true,
            ..dynamic(false)
        };
        let id = sized_entity(&mut scene, "spinner", (40.0, 60.0), (30.0, 10.0), props);
        physics.add_entity(scene.get_entity(id).unwrap());

        physics.set_angular_velocity(id, std::f32::consts::FRAC_PI_2);
        for _ in 0..30 {
            physics.step(&mut scene);
        }

        let rotation = float_attr(&scene, id, "rotation").expect("rotation attribute");
        assert!(
            (rotation - 45.0).abs() < 1.0,
            "half a second at 90°/s: {}",
            rotation
        );
        let entity = scene.get_entity(id).unwrap();
        assert!((entity.get_x() - 40.0).abs() < 1e-3);
        assert!((entity.get_y() - 60.0).abs() < 1e-3);

        let outline = physics.collider_outlines()[0];
        assert_eq!(
            outline.center,
            (55.0, 65.0),
            "collider centered on the sprite"
        );
        assert!((outline.rotation.to_degrees() - rotation).abs() < 1e-3);
    }

    #[test]
    fn test_apply_force_is_frame_rate_independent() {
        // The same force applied every frame must produce the same motion
        // at 30, 60 and 144 frames per second.
        let velocity_after_one_second = |frame_dt: f32| {
            let mut scene = Scene::new("test_scene").unwrap();
            let mut physics = PhysicsEngine::new();
            let id = sized_entity(&mut scene, "ship", (0.0, 0.0), (10.0, 10.0), dynamic(false));
            physics.add_entity(scene.get_entity(id).unwrap());
            simulate_frames(&mut physics, &mut scene, frame_dt, 1.0, |physics| {
                physics.apply_force(id, (1000.0, 0.0))
            });
            physics.get_velocity(id).unwrap().0
        };

        let at_60 = velocity_after_one_second(1.0 / 60.0);
        for frame_dt in [1.0 / 30.0, 1.0 / 144.0] {
            let other = velocity_after_one_second(frame_dt);
            assert!(
                (other - at_60).abs() / at_60 < 0.05,
                "frame_dt {}: {} vs {} at 60 fps",
                frame_dt,
                other,
                at_60
            );
        }
    }

    #[test]
    fn test_torque_does_not_accumulate() {
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics = PhysicsEngine::new();
        let props = PhysicsProperties {
            can_rotate: true,
            ..dynamic(false)
        };
        let id = sized_entity(&mut scene, "wheel", (0.0, 0.0), (20.0, 20.0), props);
        physics.add_entity(scene.get_entity(id).unwrap());

        // One frame of torque, then nothing
        physics.apply_torque(id, 5_000_000.0);
        physics.step(&mut scene);
        let spin = physics.get_angular_velocity(id).unwrap();
        assert!(spin > 0.0);
        for _ in 0..30 {
            physics.step(&mut scene);
        }
        let later = physics.get_angular_velocity(id).unwrap();
        assert!(
            (later - spin).abs() < 1e-3,
            "spin must not keep growing: {} -> {}",
            spin,
            later
        );
    }

    #[test]
    fn test_cleanup_restores_default_gravity() {
        let mut physics = PhysicsEngine::new();
        physics.set_gravity(0.0, -500.0);
        physics.cleanup();
        assert_eq!(physics.get_gravity(), DEFAULT_GRAVITY);
    }

    #[test]
    fn test_bodies_of_removed_entities_are_dropped() {
        // An entity deleted from the scene without telling the physics
        // engine must not stay behind as an invisible obstacle, nor break
        // the other entities' updates.
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics = PhysicsEngine::new();
        let ghost = sized_entity(&mut scene, "ghost", (0.0, 0.0), (10.0, 10.0), dynamic(true));
        let other = sized_entity(
            &mut scene,
            "other",
            (50.0, 0.0),
            (10.0, 10.0),
            dynamic(true),
        );
        physics.load_scene(&scene);

        scene.delete_entity(ghost).unwrap();
        for _ in 0..30 {
            physics.step(&mut scene);
        }

        assert!(!physics.has_rigid_body(ghost));
        assert_eq!(physics.rigid_body_count(), 1);
        assert!(
            scene.get_entity(other).unwrap().get_y() > 0.0,
            "others keep moving"
        );
    }

    #[test]
    fn test_set_position_teleports_body() {
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics = PhysicsEngine::new();
        let id = sized_entity(
            &mut scene,
            "player",
            (0.0, 0.0),
            (10.0, 10.0),
            dynamic(false),
        );
        physics.add_entity(scene.get_entity(id).unwrap());

        physics.set_position(id, 200.0, 150.0);
        physics.step(&mut scene);

        let entity = scene.get_entity(id).unwrap();
        assert_eq!((entity.get_x(), entity.get_y()), (200.0, 150.0));
    }

    #[test]
    fn test_gravity_field_attracts_bodies() {
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics = PhysicsEngine::new();
        physics.set_gravity(0.0, 0.0);
        let planet_props = PhysicsProperties {
            creates_gravity: true,
            ..fixed()
        };
        sized_entity(
            &mut scene,
            "planet",
            (-10.0, -10.0),
            (20.0, 20.0),
            planet_props,
        );
        let moon = sized_entity(
            &mut scene,
            "moon",
            (95.0, -5.0),
            (10.0, 10.0),
            dynamic(true),
        );
        physics.load_scene(&scene);

        for _ in 0..60 {
            physics.step(&mut scene);
        }

        // 50 px/s² at 100 px: about 25 px in one second
        let moved = 95.0 - scene.get_entity(moon).unwrap().get_x();
        assert!(
            moved > 15.0 && moved < 40.0,
            "moon moved {} px towards the planet",
            moved
        );
    }

    #[test]
    fn test_body_without_collider_still_falls() {
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics = PhysicsEngine::new();
        let props = PhysicsProperties {
            has_collision: false,
            ..dynamic(true)
        };
        let id = scene
            .create_physical_entity("ghost", (0.0, 0.0, 0.0), props)
            .unwrap();
        physics.add_entity(scene.get_entity(id).unwrap());
        assert!(!physics.has_collider(id));

        for _ in 0..60 {
            physics.step(&mut scene);
        }
        assert!(scene.get_entity(id).unwrap().get_y() > 10.0);
    }

    #[test]
    fn test_collider_shape_defaults_to_rectangle() {
        let mut scene = Scene::new("test_scene").unwrap();
        let mut physics = PhysicsEngine::new();
        let square = sized_entity(&mut scene, "crate", (0.0, 0.0), (32.0, 32.0), fixed());
        let ball = sized_entity(&mut scene, "ball", (100.0, 0.0), (40.0, 20.0), fixed());
        scene
            .get_entity_mut(ball)
            .unwrap()
            .create_attribute(
                "collider_shape",
                AttributeType::String,
                AttributeValue::String("circle".to_string()),
            )
            .unwrap();
        physics.add_entity(scene.get_entity(square).unwrap());
        physics.add_entity(scene.get_entity(ball).unwrap());

        let mut outlines = physics.collider_outlines();
        outlines.sort_by(|a, b| a.center.0.total_cmp(&b.center.0));
        assert_eq!(outlines[0].shape, ColliderShape::Rectangle);
        assert_eq!(outlines[0].size, (32.0, 32.0));
        assert_eq!(outlines[1].shape, ColliderShape::Circle);
        assert_eq!(
            outlines[1].size,
            (20.0, 20.0),
            "circles fit inside the body"
        );
    }
}
