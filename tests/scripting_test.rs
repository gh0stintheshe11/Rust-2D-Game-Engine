mod common;

#[cfg(test)]
mod tests {
    use mlua::Lua;

    #[test]
    fn test_run_simple_script() {
        // Test a simple Lua script that adds two numbers
        let lua = Lua::new();
        let script = r#"
            x = 10
            y = 20
            result = x + y
        "#;

        let result = lua.load(script).exec();
        assert!(result.is_ok(), "Failed to run a simple Lua script");
    }

    #[test]
    fn test_run_script_with_error() {
        // Test a Lua script that tries to use an undefined variable
        let lua = Lua::new();
        let script = r#"
            x = 10
            if y == nil then
                y = 0  -- Assign a default value if 'y' is undefined
            end
            result = x + y
        "#;

        let result = lua.load(script).exec();
        assert!(
            result.is_ok(),
            "Expected Lua to handle undefined variables as nil, but it failed"
        );
    }

    #[test]
    fn test_lua_math_operations() {
        // Test a Lua script performing math operations
        let script = r#"
            result = (10 * 5) / 2 - 7
        "#;

        let lua = Lua::new();
        lua.load(script).exec().unwrap();
        let result: f64 = lua.globals().get("result").unwrap();
        assert_eq!(result, 18.0, "Math operation failed in Lua");
    }

    #[test]
    fn test_pass_data_to_lua() {
        // Test passing data to Lua
        let lua = Lua::new();
        let globals = lua.globals();
        globals.set("x", 50).unwrap();
        globals.set("y", 100).unwrap();

        lua.load(
            r#"
            result = x + y
        "#,
        )
        .exec()
        .unwrap();

        let result: i32 = lua.globals().get("result").unwrap();
        assert_eq!(result, 150, "Failed to pass data to Lua script");
    }

    #[test]
    fn test_return_data_from_lua() {
        // Test returning data from Lua to Rust
        let lua = Lua::new();
        lua.load(
            r#"
            function add(a, b)
                return a + b
            end
        "#,
        )
        .exec()
        .unwrap();

        let add: mlua::Function = lua.globals().get("add").unwrap();
        let result: i32 = add.call((10, 20)).unwrap();
        assert_eq!(result, 30, "Failed to return correct data from Lua");
    }

    #[test]
    fn test_complex_script() {
        // Test running a more complex Lua script (basic object simulation)
        let script = r#"
            obj = {
                x = 0,
                y = 0,
                vx = 1,
                vy = 1
            }

            function update_position(obj)
                obj.x = obj.x + obj.vx
                obj.y = obj.y + obj.vy
            end

            update_position(obj)
        "#;

        let lua = Lua::new();
        lua.load(script).exec().unwrap();

        let obj: mlua::Table = lua.globals().get("obj").unwrap();
        let x: i32 = obj.get("x").unwrap();
        let y: i32 = obj.get("y").unwrap();

        assert_eq!(x, 1, "Object x-coordinate was not updated correctly");
        assert_eq!(y, 1, "Object y-coordinate was not updated correctly");
    }

    #[test]
    fn test_handle_error_in_lua_script() {
        // Test Lua's handling of division by zero
        let lua = Lua::new();
        let script = r#"
            function divide(a, b)
                return a / b
            end

            result = divide(10, 0)
        "#;

        lua.load(script).exec().unwrap();
        let result: f64 = lua.globals().get("result").unwrap();
        assert!(
            result.is_infinite(),
            "Expected Lua to return 'inf' on division by zero, but got: {}",
            result
        );
    }
}

/// Integration tests for the engine's scripting session
/// (persistent VM, per-script environments, safe engine bindings).
#[cfg(test)]
mod session_tests {
    use rust_2d_game_engine::audio::AudioEngine;
    use rust_2d_game_engine::input::InputHandler;
    use rust_2d_game_engine::physics::PhysicsEngine;
    use rust_2d_game_engine::scene::SceneManager;
    use rust_2d_game_engine::scripting::LuaScripting;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::rc::Rc;

    struct TestSession {
        lua: LuaScripting,
        scene_manager: Rc<RefCell<SceneManager>>,
        physics: Rc<RefCell<PhysicsEngine>>,
        audio: Rc<RefCell<AudioEngine>>,
        scene_id: uuid::Uuid,
        script_dir: PathBuf,
    }

    fn setup(test_name: &str) -> TestSession {
        setup_with_audio(test_name, AudioEngine::new())
    }

    fn setup_with_audio(test_name: &str, audio_engine: AudioEngine) -> TestSession {
        let script_dir = std::env::temp_dir().join(format!(
            "rust2d_engine_test_{}_{}",
            test_name,
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&script_dir).unwrap();

        let mut manager = SceneManager::new();
        let scene_id = manager.create_scene("test_scene").unwrap();
        manager.set_active_scene(scene_id).unwrap();

        let scene_manager = Rc::new(RefCell::new(manager));
        let physics_engine = Rc::new(RefCell::new(PhysicsEngine::new()));
        let input_handler = Rc::new(RefCell::new(InputHandler::new()));
        let audio_engine = Rc::new(RefCell::new(audio_engine));

        let mut lua = LuaScripting::new();
        lua.start_session(
            Rc::clone(&scene_manager),
            Rc::clone(&physics_engine),
            Rc::clone(&input_handler),
            Rc::clone(&audio_engine),
        )
        .expect("Failed to start Lua session");

        TestSession {
            lua,
            scene_manager,
            physics: physics_engine,
            audio: audio_engine,
            scene_id,
            script_dir,
        }
    }

    fn add_scripted_entity(session: &TestSession, name: &str, script_source: &str) -> uuid::Uuid {
        let script_path = session.script_dir.join(format!("{}.lua", name));
        std::fs::write(&script_path, script_source).unwrap();

        let mut manager = session.scene_manager.borrow_mut();
        let scene = manager.get_scene_mut(session.scene_id).unwrap();
        let entity_id = scene.create_entity(name).unwrap();
        scene
            .get_entity_mut(entity_id)
            .unwrap()
            .set_script(script_path)
            .unwrap();
        entity_id
    }

    #[test]
    fn test_script_state_persists_across_frames() {
        let mut session = setup("state_persists");
        add_scripted_entity(
            &session,
            "counter",
            r#"
            function update(scene_id, entity_id)
                if script_state.state.count == nil then
                    script_state.state.count = 0
                end
                script_state.state.count = script_state.state.count + 1
            end
            "#,
        );

        for _ in 0..3 {
            session.lua.run_scripts_for_scene(session.scene_id).unwrap();
        }

        let count: i64 = session
            .lua
            .lua()
            .load("return script_state.state.count")
            .eval()
            .unwrap();
        assert_eq!(count, 3, "script_state should persist across frames");
    }

    #[test]
    fn test_script_can_spawn_entities_mid_frame() {
        let mut session = setup("spawn_mid_frame");
        add_scripted_entity(
            &session,
            "spawner",
            r#"
            function update(scene_id, entity_id)
                local id = add_entity(scene_id, "spawned_" .. tostring(math.random(1, 100000)))
                set_x(scene_id, id, 42.0)
            end
            "#,
        );

        session.lua.run_scripts_for_scene(session.scene_id).unwrap();
        session.lua.run_scripts_for_scene(session.scene_id).unwrap();

        let manager = session.scene_manager.borrow();
        let scene = manager.get_scene(session.scene_id).unwrap();
        // 1 default camera + 1 spawner + 2 spawned entities
        assert_eq!(
            scene.entities.len(),
            4,
            "Scripts should be able to add entities while running"
        );
    }

    #[test]
    fn test_failing_script_does_not_block_others() {
        let mut session = setup("error_isolation");
        add_scripted_entity(
            &session,
            "a_broken",
            r#"
            function update(scene_id, entity_id)
                error("intentional failure")
            end
            "#,
        );
        add_scripted_entity(
            &session,
            "b_working",
            r#"
            function update(scene_id, entity_id)
                script_state.state.worked = true
            end
            "#,
        );

        session.lua.run_scripts_for_scene(session.scene_id).unwrap();

        let worked: bool = session
            .lua
            .lua()
            .load("return script_state.state.worked == true")
            .eval()
            .unwrap();
        assert!(
            worked,
            "A failing script must not prevent later scripts from running"
        );
    }

    #[test]
    fn test_scripts_have_isolated_update_functions() {
        let mut session = setup("env_isolation");
        // Both scripts define update(); with shared globals the second would
        // overwrite the first. Each must run its own version.
        add_scripted_entity(
            &session,
            "first",
            r#"
            function update(scene_id, entity_id)
                script_state.state.first = (script_state.state.first or 0) + 1
            end
            "#,
        );
        add_scripted_entity(
            &session,
            "second",
            r#"
            function update(scene_id, entity_id)
                script_state.state.second = (script_state.state.second or 0) + 1
            end
            "#,
        );

        session.lua.run_scripts_for_scene(session.scene_id).unwrap();

        let (first, second): (i64, i64) = session
            .lua
            .lua()
            .load("return script_state.state.first or 0, script_state.state.second or 0")
            .eval()
            .unwrap();
        assert_eq!(first, 1, "first script's update() should have run");
        assert_eq!(second, 1, "second script's update() should have run");
    }

    #[test]
    fn test_collision_query_and_end_game() {
        use rust_2d_game_engine::scene::PhysicsProperties;

        let mut session = setup("collision_end_game");

        // Two physical entities: a fixed obstacle and a player dropped onto
        // it from above. (Spawning bodies deeply overlapped is degenerate -
        // the solver can produce NaN positions - so simulate a real fall.)
        let (obstacle_id, player_id) = {
            let mut manager = session.scene_manager.borrow_mut();
            let scene = manager.get_scene_mut(session.scene_id).unwrap();
            let obstacle_id = scene
                .create_physical_entity(
                    "obstacle",
                    (0.0, 0.0, 0.0),
                    PhysicsProperties {
                        is_movable: false,
                        has_collision: true,
                        ..Default::default()
                    },
                )
                .unwrap();
            let player_id = scene
                .create_physical_entity(
                    "player",
                    // Directly above the obstacle; +Y is down, so it falls onto it
                    (0.0, -3.0, 0.0),
                    PhysicsProperties {
                        is_movable: true,
                        affected_by_gravity: true,
                        has_collision: true,
                        ..Default::default()
                    },
                )
                .unwrap();
            (obstacle_id, player_id)
        };

        // The player's script ends the game when touching the obstacle
        let script_path = session.script_dir.join("player.lua");
        std::fs::write(
            &script_path,
            r#"
            function update(scene_id, entity_id)
                local hits = get_colliding_entities(entity_id)
                for i = 1, #hits do
                    if get_entity_name(scene_id, hits[i]) == "obstacle" then
                        end_game()
                    end
                end
            end
            "#,
        )
        .unwrap();
        {
            let mut manager = session.scene_manager.borrow_mut();
            let scene = manager.get_scene_mut(session.scene_id).unwrap();
            scene
                .get_entity_mut(player_id)
                .unwrap()
                .set_script(script_path)
                .unwrap();
        }

        // Register bodies and step until the player lands on the obstacle
        {
            let mut manager = session.scene_manager.borrow_mut();
            let scene = manager.get_scene_mut(session.scene_id).unwrap();
            let mut physics = session.physics.borrow_mut();
            physics.add_entity(scene.get_entity(obstacle_id).unwrap());
            physics.add_entity(scene.get_entity(player_id).unwrap());

            let mut touched = false;
            for _ in 0..240 {
                physics.step(scene);
                if physics
                    .get_colliding_entities(player_id)
                    .contains(&obstacle_id)
                {
                    touched = true;
                    break;
                }
            }
            assert!(touched, "player should land on the obstacle within 4s");
        }

        assert!(
            !session.lua.take_game_stop_request(),
            "no stop should be requested before scripts run"
        );

        session.lua.run_scripts_for_scene(session.scene_id).unwrap();

        assert!(
            session.lua.take_game_stop_request(),
            "script should detect the collision and request end_game"
        );
        assert!(
            !session.lua.take_game_stop_request(),
            "the stop request flag must clear after being taken"
        );
    }

    #[test]
    fn test_generic_attribute_access() {
        use rust_2d_game_engine::scene::{AttributeType, AttributeValue};

        let mut session = setup("attribute_access");

        // An entity with designer-defined attributes of each kind
        let entity_id = {
            let mut manager = session.scene_manager.borrow_mut();
            let scene = manager.get_scene_mut(session.scene_id).unwrap();
            let id = scene.create_entity("player").unwrap();
            let entity = scene.get_entity_mut(id).unwrap();
            entity
                .create_attribute("speed", AttributeType::Float, AttributeValue::Float(10.0))
                .unwrap();
            entity
                .create_attribute(
                    "alive",
                    AttributeType::Boolean,
                    AttributeValue::Boolean(true),
                )
                .unwrap();
            entity
                .create_attribute(
                    "title",
                    AttributeType::String,
                    AttributeValue::String("rookie".into()),
                )
                .unwrap();
            entity
                .create_attribute(
                    "spawn",
                    AttributeType::Vector2,
                    AttributeValue::Vector2(3.0, 4.0),
                )
                .unwrap();
            id
        };

        add_scripted_entity(
            &session,
            "logic",
            r#"
            function update(scene_id, entity_id)
                -- read every type, transform, write back
                local target = script_state.state.target
                local speed = get_attribute(scene_id, target, "speed")
                set_attribute(scene_id, target, "speed", speed * 2)
                set_attribute(scene_id, target, "alive", false)
                set_attribute(scene_id, target, "title", "veteran")
                local spawn = get_attribute(scene_id, target, "spawn")
                set_attribute(scene_id, target, "spawn", { x = spawn.x + 1, y = spawn.y + 1 })

                -- built-in x attribute is readable/writable too
                set_attribute(scene_id, target, "x", 42.0)

                -- probing
                script_state.state.has_speed = has_attribute(scene_id, target, "speed")
                script_state.state.has_nope = has_attribute(scene_id, target, "nope")
                script_state.state.missing = get_attribute(scene_id, target, "nope") == nil
            end
            "#,
        );

        session
            .lua
            .lua()
            .load(format!("script_state.state.target = '{}'", entity_id))
            .exec()
            .unwrap();

        session.lua.run_scripts_for_scene(session.scene_id).unwrap();

        let manager = session.scene_manager.borrow();
        let scene = manager.get_scene(session.scene_id).unwrap();
        let entity = scene.get_entity(entity_id).unwrap();
        assert_eq!(
            entity.get_attribute_by_name("speed").unwrap().value,
            AttributeValue::Float(20.0)
        );
        assert_eq!(
            entity.get_attribute_by_name("alive").unwrap().value,
            AttributeValue::Boolean(false)
        );
        assert_eq!(
            entity.get_attribute_by_name("title").unwrap().value,
            AttributeValue::String("veteran".into())
        );
        assert_eq!(
            entity.get_attribute_by_name("spawn").unwrap().value,
            AttributeValue::Vector2(4.0, 5.0)
        );
        assert_eq!(entity.get_x(), 42.0);

        let (has_speed, has_nope, missing): (bool, bool, bool) = session
            .lua
            .lua()
            .load(
                "return script_state.state.has_speed, script_state.state.has_nope, \
                 script_state.state.missing",
            )
            .eval()
            .unwrap();
        assert!(has_speed);
        assert!(!has_nope);
        assert!(missing, "get_attribute on a missing attribute returns nil");
    }

    #[test]
    fn test_set_attribute_type_mismatch_errors() {
        use rust_2d_game_engine::scene::{AttributeType, AttributeValue};

        let mut session = setup("attribute_type_mismatch");

        let entity_id = {
            let mut manager = session.scene_manager.borrow_mut();
            let scene = manager.get_scene_mut(session.scene_id).unwrap();
            let id = scene.create_entity("typed").unwrap();
            scene
                .get_entity_mut(id)
                .unwrap()
                .create_attribute("speed", AttributeType::Float, AttributeValue::Float(1.0))
                .unwrap();
            id
        };

        add_scripted_entity(
            &session,
            "bad_write",
            r#"
            function update(scene_id, entity_id)
                local ok = pcall(set_attribute, scene_id, script_state.state.target, "speed", "fast")
                script_state.state.write_rejected = not ok
            end
            "#,
        );

        session
            .lua
            .lua()
            .load(format!("script_state.state.target = '{}'", entity_id))
            .exec()
            .unwrap();
        session.lua.run_scripts_for_scene(session.scene_id).unwrap();

        let rejected: bool = session
            .lua
            .lua()
            .load("return script_state.state.write_rejected == true")
            .eval()
            .unwrap();
        assert!(
            rejected,
            "writing a string into a Float attribute must fail"
        );

        let manager = session.scene_manager.borrow();
        let scene = manager.get_scene(session.scene_id).unwrap();
        assert_eq!(
            scene
                .get_entity(entity_id)
                .unwrap()
                .get_attribute_by_name("speed")
                .unwrap()
                .value,
            AttributeValue::Float(1.0),
            "the attribute must be unchanged after a rejected write"
        );
    }

    #[test]
    fn test_init_runs_once_before_update() {
        let mut session = setup("init_hook");
        add_scripted_entity(
            &session,
            "lifecycle",
            r#"
            function init(scene_id, entity_id)
                script_state.state.inits = (script_state.state.inits or 0) + 1
                script_state.state.init_before_update = script_state.state.updates == nil
            end

            function update(scene_id, entity_id)
                script_state.state.updates = (script_state.state.updates or 0) + 1
            end
            "#,
        );

        for _ in 0..3 {
            session.lua.run_scripts_for_scene(session.scene_id).unwrap();
        }

        let (inits, updates, ordered): (i64, i64, bool) = session
            .lua
            .lua()
            .load(
                "return script_state.state.inits, script_state.state.updates, \
                 script_state.state.init_before_update == true",
            )
            .eval()
            .unwrap();
        assert_eq!(inits, 1, "init() must run exactly once per entity");
        assert_eq!(updates, 3, "update() must run every frame");
        assert!(ordered, "init() must run before the first update()");
    }

    #[test]
    fn test_on_collision_fires_once_per_contact() {
        use rust_2d_game_engine::scene::PhysicsProperties;

        let mut session = setup("on_collision_hook");

        // A fixed obstacle and a player dropped onto it
        let (obstacle_id, player_id) = {
            let mut manager = session.scene_manager.borrow_mut();
            let scene = manager.get_scene_mut(session.scene_id).unwrap();
            let obstacle_id = scene
                .create_physical_entity(
                    "obstacle",
                    (0.0, 0.0, 0.0),
                    PhysicsProperties {
                        is_movable: false,
                        has_collision: true,
                        ..Default::default()
                    },
                )
                .unwrap();
            let player_id = scene
                .create_physical_entity(
                    "player",
                    (0.0, -3.0, 0.0),
                    PhysicsProperties {
                        is_movable: true,
                        affected_by_gravity: true,
                        has_collision: true,
                        ..Default::default()
                    },
                )
                .unwrap();
            (obstacle_id, player_id)
        };

        let script_path = session.script_dir.join("collider.lua");
        std::fs::write(
            &script_path,
            r#"
            function update(scene_id, entity_id) end

            function on_collision(scene_id, entity_id, other_id)
                script_state.state.hits = (script_state.state.hits or 0) + 1
                script_state.state.last_other = get_entity_name(scene_id, other_id)
            end
            "#,
        )
        .unwrap();
        {
            let mut manager = session.scene_manager.borrow_mut();
            let scene = manager.get_scene_mut(session.scene_id).unwrap();
            scene
                .get_entity_mut(player_id)
                .unwrap()
                .set_script(script_path)
                .unwrap();
        }

        {
            let mut manager = session.scene_manager.borrow_mut();
            let scene = manager.get_scene_mut(session.scene_id).unwrap();
            let mut physics = session.physics.borrow_mut();
            physics.add_entity(scene.get_entity(obstacle_id).unwrap());
            physics.add_entity(scene.get_entity(player_id).unwrap());
        }

        // Simulate frames like the runtime: scripts, physics, collision hooks
        for _ in 0..240 {
            session.lua.run_scripts_for_scene(session.scene_id).unwrap();
            {
                let mut manager = session.scene_manager.borrow_mut();
                let scene = manager.get_scene_mut(session.scene_id).unwrap();
                session.physics.borrow_mut().step(scene);
            }
            session
                .lua
                .dispatch_collision_events(session.scene_id)
                .unwrap();
        }

        let (hits, other): (i64, String) = session
            .lua
            .lua()
            .load("return script_state.state.hits or 0, script_state.state.last_other or ''")
            .eval()
            .unwrap();
        assert_eq!(
            hits, 1,
            "on_collision must fire once when contact begins, not every frame"
        );
        assert_eq!(other, "obstacle");
    }

    #[test]
    fn test_audio_bindings_fail_gracefully() {
        let mut session = setup("audio_bindings");
        add_scripted_entity(
            &session,
            "noisy",
            r#"
            function update(scene_id, entity_id)
                -- Missing file (and possibly no audio device): must return
                -- nil rather than raise an error
                script_state.state.play_result = play_sound("assets/sounds/missing.ogg")
                script_state.state.survived = true
                stop_all_sounds()
            end
            "#,
        );

        session.lua.run_scripts_for_scene(session.scene_id).unwrap();

        let (survived, silent): (bool, bool) = session
            .lua
            .lua()
            .load(
                "return script_state.state.survived == true, \
                 script_state.state.play_result == nil",
            )
            .eval()
            .unwrap();
        assert!(survived, "audio calls must not raise errors");
        assert!(silent, "unplayable sound returns nil");
    }

    #[test]
    fn test_create_physical_entity_applies_position() {
        let mut session = setup("cpe_position");
        add_scripted_entity(
            &session,
            "spawner",
            r#"
            function update(scene_id, entity_id)
                if script_state.state.spawned == nil then
                    script_state.state.spawned =
                        create_physical_entity(scene_id, "box", 12.0, 34.0, 2.0)
                end
            end
            "#,
        );

        session.lua.run_scripts_for_scene(session.scene_id).unwrap();

        let spawned: String = session
            .lua
            .lua()
            .load("return script_state.state.spawned")
            .eval()
            .unwrap();
        let spawned_id = uuid::Uuid::parse_str(&spawned).unwrap();

        let manager = session.scene_manager.borrow();
        let scene = manager.get_scene(session.scene_id).unwrap();
        let entity = scene.get_entity(spawned_id).unwrap();
        assert_eq!(entity.get_x(), 12.0, "spawn x must be applied");
        assert_eq!(entity.get_y(), 34.0, "spawn y must be applied");
        assert_eq!(entity.get_z(), 2.0, "spawn z must be applied");
    }

    #[test]
    fn test_remove_entity_also_removes_physics_body() {
        let mut session = setup("remove_body");
        add_scripted_entity(
            &session,
            "spawner",
            r#"
            function update(scene_id, entity_id)
                if script_state.state.box == nil then
                    local box = create_physical_entity(scene_id, "box", 0.0, 0.0, 0.0)
                    add_entity_to_physics_engine(box)
                    script_state.state.box = box
                else
                    remove_entity(scene_id, script_state.state.box)
                end
            end
            "#,
        );

        session.lua.run_scripts_for_scene(session.scene_id).unwrap();
        assert_eq!(session.physics.borrow().rigid_body_count(), 1);

        session.lua.run_scripts_for_scene(session.scene_id).unwrap();
        assert_eq!(
            session.physics.borrow().rigid_body_count(),
            0,
            "the body must go away with the entity"
        );
    }

    #[test]
    fn test_position_setters_move_physics_bodies() {
        let mut session = setup("teleport");
        add_scripted_entity(
            &session,
            "teleporter",
            r#"
            function update(scene_id, entity_id)
                if script_state.state.box == nil then
                    local box = create_physical_entity(scene_id, "box", 0.0, 0.0, 0.0)
                    add_entity_to_physics_engine(box)
                    script_state.state.box = box
                else
                    set_position(scene_id, script_state.state.box, 300.0, 40.0)
                end
            end
            "#,
        );

        // Frame 1 spawns, frame 2 teleports; the physics step afterwards
        // must keep the new position instead of snapping back
        session.lua.run_scripts_for_scene(session.scene_id).unwrap();
        session.lua.run_scripts_for_scene(session.scene_id).unwrap();
        {
            let mut manager = session.scene_manager.borrow_mut();
            let scene = manager.get_scene_mut(session.scene_id).unwrap();
            session.physics.borrow_mut().step(scene);
        }

        let box_id: String = session
            .lua
            .lua()
            .load("return script_state.state.box")
            .eval()
            .unwrap();
        let box_id = uuid::Uuid::parse_str(&box_id).unwrap();
        let manager = session.scene_manager.borrow();
        let entity = manager
            .get_scene(session.scene_id)
            .unwrap()
            .get_entity(box_id)
            .unwrap();
        assert!(
            (entity.get_x() - 300.0).abs() < 1.0,
            "x = {}",
            entity.get_x()
        );
        assert!(
            entity.get_y() >= 40.0 && entity.get_y() < 41.0,
            "y = {}",
            entity.get_y()
        );
    }

    fn console_contains(text: &str) -> bool {
        rust_2d_game_engine::logger::LOGGER
            .get_console_messages()
            .iter()
            .any(|message| message.text.contains(text))
    }

    /// Rewrite a script file and push its modification time forward, so the
    /// change is detected even on coarse-grained file systems.
    fn rewrite_script(session: &TestSession, name: &str, source: &str) {
        let path = session.script_dir.join(format!("{}.lua", name));
        std::fs::write(&path, source).unwrap();
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(10);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(later)
            .unwrap();
    }

    fn state<T: mlua::FromLua>(session: &TestSession, key: &str) -> T {
        session
            .lua
            .lua()
            .load(format!("return script_state.state.{}", key))
            .eval()
            .unwrap()
    }

    #[test]
    fn test_sandbox_hides_unsafe_libraries() {
        let mut session = setup("sandbox");
        add_scripted_entity(
            &session,
            "probe",
            r#"
            function update(scene_id, entity_id)
                local s = script_state.state
                s.io = io == nil
                s.execute = os.execute == nil
                s.remove = os.remove == nil
                s.exit = os.exit == nil
                s.require = require == nil
                s.dofile = dofile == nil
                s.clock = type(os.clock()) == "number"
                s.binary = load(string.dump(function() end)) == nil
            end
            "#,
        );
        session.lua.run_scripts_for_scene(session.scene_id).unwrap();

        for key in [
            "io", "execute", "remove", "exit", "require", "dofile", "clock", "binary",
        ] {
            assert!(
                state::<bool>(&session, key),
                "sandbox check failed: {}",
                key
            );
        }
    }

    #[test]
    fn test_runaway_scripts_are_aborted() {
        let mut session = setup("runaway");
        add_scripted_entity(
            &session,
            "a_loop",
            "function update(scene_id, entity_id) while true do end end",
        );
        add_scripted_entity(
            &session,
            "b_coroutine_loop",
            r#"
            function update(scene_id, entity_id)
                coroutine.wrap(function() while true do end end)()
            end
            "#,
        );
        add_scripted_entity(
            &session,
            "c_survivor",
            "function update(scene_id, entity_id) script_state.state.alive = true end",
        );

        let started = std::time::Instant::now();
        session.lua.run_scripts_for_scene(session.scene_id).unwrap();
        assert!(started.elapsed() < std::time::Duration::from_secs(10));
        assert!(state::<bool>(&session, "alive"), "other scripts still run");
        assert!(console_contains("infinite loop"));
    }

    #[test]
    fn test_set_attribute_never_reenters_lua_while_borrowed() {
        // A table whose __index calls back into the engine must not panic
        // with "already borrowed": the value is read before borrowing
        let mut session = setup("reentrant");
        let entity_id = add_scripted_entity(
            &session,
            "reentrant",
            r#"
            function update(scene_id, entity_id)
                local sneaky = setmetatable({}, {__index = function()
                    return get_attribute(scene_id, entity_id, "x")
                end})
                script_state.state.ok = pcall(set_attribute, scene_id, entity_id, "spawn", sneaky)
                set_attribute(scene_id, entity_id, "spawn", {x = 3, y = 4})
            end
            "#,
        );
        session
            .scene_manager
            .borrow_mut()
            .get_scene_mut(session.scene_id)
            .unwrap()
            .get_entity_mut(entity_id)
            .unwrap()
            .create_attribute(
                "spawn",
                rust_2d_game_engine::scene::AttributeType::Vector2,
                rust_2d_game_engine::scene::AttributeValue::Vector2(0.0, 0.0),
            )
            .unwrap();

        session.lua.run_scripts_for_scene(session.scene_id).unwrap();

        assert!(
            !state::<bool>(&session, "ok"),
            "a non-vector table is rejected"
        );
        let manager = session.scene_manager.borrow();
        let entity = manager
            .get_scene(session.scene_id)
            .unwrap()
            .get_entity(entity_id)
            .unwrap();
        assert_eq!(
            entity.get_attribute_by_name("spawn").unwrap().value,
            rust_2d_game_engine::scene::AttributeValue::Vector2(3.0, 4.0)
        );
    }

    #[test]
    fn test_hot_reload_keeps_script_variables() {
        let mut session = setup("hot_reload");
        add_scripted_entity(
            &session,
            "mover",
            r#"
            function init(scene_id, entity_id) speed = 5 end
            function update(scene_id, entity_id) script_state.state.seen = speed end
            "#,
        );
        session.lua.run_scripts_for_scene(session.scene_id).unwrap();
        assert_eq!(state::<i64>(&session, "seen"), 5);

        // Edited while playing: init doesn't run again, but `speed` survives
        rewrite_script(
            &session,
            "mover",
            r#"
            function init(scene_id, entity_id) speed = 5 end
            function update(scene_id, entity_id) script_state.state.seen = speed * 2 end
            "#,
        );
        session.lua.run_scripts_for_scene(session.scene_id).unwrap();
        assert_eq!(state::<i64>(&session, "seen"), 10);

        // A broken edit keeps the last good version running
        rewrite_script(&session, "mover", "function update(scene_id, entity_id");
        session.lua.run_scripts_for_scene(session.scene_id).unwrap();
        session.lua.run_scripts_for_scene(session.scene_id).unwrap();
        assert_eq!(state::<i64>(&session, "seen"), 10);
        assert!(console_contains("The previous version keeps running"));
    }

    #[test]
    fn test_failed_init_pauses_the_entity() {
        let mut session = setup("init_fails");
        add_scripted_entity(
            &session,
            "fragile",
            r#"
            function init(scene_id, entity_id) error("boom") end
            function update(scene_id, entity_id)
                script_state.state.updates = (script_state.state.updates or 0) + 1
            end
            "#,
        );
        for _ in 0..3 {
            session.lua.run_scripts_for_scene(session.scene_id).unwrap();
        }
        let updates: Option<i64> = state(&session, "updates");
        assert_eq!(updates, None, "update must not run after a failed init");
    }

    #[test]
    fn test_print_goes_to_the_console() {
        let mut session = setup("print");
        add_scripted_entity(
            &session,
            "talker",
            r#"function update(scene_id, entity_id) print("hello from lua", 42, nil) end"#,
        );
        session.lua.run_scripts_for_scene(session.scene_id).unwrap();
        assert!(console_contains("hello from lua\t42\tnil"));
    }

    #[test]
    fn test_time_globals_and_lenient_input_names() {
        let mut session = setup("time_input");
        add_scripted_entity(
            &session,
            "reader",
            r#"
            function update(scene_id, entity_id)
                script_state.state.dt = delta_time
                script_state.state.t = accumulated_time
                script_state.state.space = is_key_pressed("space")
                script_state.state.mouse = is_mouse_pressed("MouseLeft")
                script_state.state.stopped = stop_sound(nil) == nil and not is_sound_playing(nil)
            end
            "#,
        );
        session.lua.update_global_time(0.25).unwrap();
        session.lua.update_global_time(0.5).unwrap();
        session.lua.run_scripts_for_scene(session.scene_id).unwrap();

        assert_eq!(state::<f64>(&session, "dt"), 0.5);
        assert_eq!(state::<f64>(&session, "t"), 0.75);
        assert!(!state::<bool>(&session, "space"));
        assert!(!state::<bool>(&session, "mouse"));
        assert!(
            state::<bool>(&session, "stopped"),
            "nil play ids are accepted"
        );
    }

    #[test]
    fn test_sound_volume_and_looping_bindings() {
        use crate::common::{TONE_PEAK, assert_level, level, offline_audio, render, write_tone};

        let (engine, mut output) = offline_audio();
        let session = setup_with_audio("sound_options", engine);
        let lua = session.lua.lua();
        let tone = write_tone(0.1).to_string_lossy().into_owned();
        lua.globals().set("TONE", tone).unwrap();

        lua.load("MUSIC = play_sound(TONE, {loop = true, volume = 0.5})")
            .exec()
            .unwrap();
        assert_level(level(&mut output), 0.5 * TONE_PEAK);

        // Looping: still playing long after the 0.1 s file ended
        render(&mut output, 0.5);
        session.audio.borrow_mut().update();
        let playing: bool = lua.load("return is_sound_playing(MUSIC)").eval().unwrap();
        assert!(playing);

        lua.load("set_sound_volume(MUSIC, 0.2); set_master_volume(0.5)")
            .exec()
            .unwrap();
        assert_level(level(&mut output), 0.1 * TONE_PEAK);

        // Bad options are errors, not silently ignored
        let (typo_ok, typo_error, negative_ok, nan_ok): (bool, String, bool, bool) = lua
            .load(
                r#"
                local typo_ok, typo_error = pcall(play_sound, TONE, {looping = true})
                local negative_ok = pcall(play_sound, TONE, {volume = -1})
                local nan_ok = pcall(set_master_volume, 0/0)
                return typo_ok, tostring(typo_error), negative_ok, nan_ok
                "#,
            )
            .eval()
            .unwrap();
        assert!(
            !typo_ok && typo_error.contains("unknown option 'looping'"),
            "{}",
            typo_error
        );
        assert!(!negative_ok && !nan_ok);

        // nil play ids (sounds that couldn't play) are accepted
        let playing: bool = lua
            .load(
                "stop_sound(MUSIC); stop_sound(nil); set_sound_volume(nil, 1)
                 return is_sound_playing(MUSIC)",
            )
            .eval()
            .unwrap();
        assert!(!playing);
        assert!(level(&mut output) < 1e-6);
    }

    #[test]
    fn test_draw_text_collects_one_frame_of_text() {
        use rust_2d_game_engine::render::{HudText, TextAlign};

        let mut session = setup("draw_text");
        add_scripted_entity(
            &session,
            "hud",
            r##"
            function update(scene_id, entity_id)
                local frame = (script_state.state.frame or 0) + 1
                script_state.state.frame = frame
                if frame == 1 then
                    draw_text("Score: " .. 3, 10, 20, {size = 32, color = "#ff000080", align = "center"})
                    draw_text(42, 0, 0)
                end
            end
            "##,
        );

        session.lua.run_scripts_for_scene(session.scene_id).unwrap();
        {
            let texts = session.lua.hud_texts();
            assert_eq!(texts.len(), 2);
            assert_eq!(
                texts[0],
                HudText {
                    text: "Score: 3".to_string(),
                    position: (10.0, 20.0),
                    size: 32.0,
                    color: egui::Color32::from_rgba_unmultiplied(255, 0, 0, 128),
                    align: TextAlign::Center,
                }
            );
            assert_eq!(texts[1].text, "42", "numbers are converted");
            assert_eq!(texts[1].size, 24.0);
            assert_eq!(texts[1].color, egui::Color32::WHITE);
        }

        // Immediate mode: the next frame starts with no text
        session.lua.run_scripts_for_scene(session.scene_id).unwrap();
        assert!(session.lua.hud_texts().is_empty());

        // Mistakes are errors, not silently ignored
        let failures: Vec<bool> = session
            .lua
            .lua()
            .load(
                r##"
                return {
                    (pcall(draw_text, "x", 0, 0, {colour = "#ffffff"})),
                    (pcall(draw_text, "x", 0, 0, {color = "red"})),
                    (pcall(draw_text, "x", 0, 0, {size = 0})),
                    (pcall(draw_text, "x", 0, 0, {align = "middle"})),
                    (pcall(draw_text, "x", 0/0, 0)),
                }
                "##,
            )
            .eval()
            .unwrap();
        assert_eq!(failures, vec![false; 5]);
        assert!(session.lua.hud_texts().is_empty());
    }

    #[test]
    fn test_play_animation_binding() {
        use rust_2d_game_engine::scene::AttributeValue;

        let session = setup("play_animation");
        let runner = session
            .scene_manager
            .borrow_mut()
            .get_scene_mut(session.scene_id)
            .unwrap()
            .create_entity("runner")
            .unwrap();
        let lua = session.lua.lua();
        lua.globals()
            .set("SCENE", session.scene_id.to_string())
            .unwrap();
        lua.globals().set("RUNNER", runner.to_string()).unwrap();
        let value = |name: &str| {
            let manager = session.scene_manager.borrow();
            let entity = manager
                .get_scene(session.scene_id)
                .unwrap()
                .get_entity(runner)
                .unwrap();
            entity
                .get_attribute_by_name(name)
                .ok()
                .map(|attr| attr.value.clone())
        };
        let set_time = |time: f32| {
            session
                .scene_manager
                .borrow_mut()
                .get_scene_mut(session.scene_id)
                .unwrap()
                .get_entity_mut(runner)
                .unwrap()
                .set_or_create_attribute("animation_time", AttributeValue::Float(time))
                .unwrap();
        };

        lua.load(r#"play_animation(SCENE, RUNNER, "0-3", {fps = 12})"#)
            .exec()
            .unwrap();
        assert_eq!(
            value("animation"),
            Some(AttributeValue::String("0-3".into()))
        );
        assert_eq!(value("animation_fps"), Some(AttributeValue::Float(12.0)));
        assert_eq!(value("animation_time"), Some(AttributeValue::Float(0.0)));
        assert_eq!(value("animation_loop"), None, "only given options are set");

        // Playing the current animation again keeps it running...
        set_time(0.3);
        lua.load(r#"play_animation(SCENE, RUNNER, "0-3")"#)
            .exec()
            .unwrap();
        assert_eq!(value("animation_time"), Some(AttributeValue::Float(0.3)));
        // ...unless asked to start over, or the frames change
        lua.load(r#"play_animation(SCENE, RUNNER, "0-3", {restart = true})"#)
            .exec()
            .unwrap();
        assert_eq!(value("animation_time"), Some(AttributeValue::Float(0.0)));
        set_time(0.3);
        lua.load(r#"play_animation(SCENE, RUNNER, "4-7", {loop = false})"#)
            .exec()
            .unwrap();
        assert_eq!(value("animation_time"), Some(AttributeValue::Float(0.0)));
        assert_eq!(
            value("animation_loop"),
            Some(AttributeValue::Boolean(false))
        );

        // Mistakes are errors
        let failures: Vec<bool> = lua
            .load(
                r#"
                return {
                    (pcall(play_animation, SCENE, RUNNER, "run")),
                    (pcall(play_animation, SCENE, RUNNER, "0-3", {speed = 2})),
                    (pcall(play_animation, SCENE, RUNNER, "0-3", {fps = 0})),
                    (pcall(play_animation, SCENE, RUNNER, "0-3", {loop = "yes"})),
                }
                "#,
            )
            .eval()
            .unwrap();
        assert_eq!(failures, vec![false; 4]);
        assert_eq!(
            value("animation"),
            Some(AttributeValue::String("4-7".into()))
        );
    }
}
