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
    use rust_2d_game_engine::ecs::SceneManager;
    use rust_2d_game_engine::input_handler::InputHandler;
    use rust_2d_game_engine::lua_scripting::LuaScripting;
    use rust_2d_game_engine::physics_engine::PhysicsEngine;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::rc::Rc;

    struct TestSession {
        lua: LuaScripting,
        scene_manager: Rc<RefCell<SceneManager>>,
        physics: Rc<RefCell<PhysicsEngine>>,
        scene_id: uuid::Uuid,
        script_dir: PathBuf,
    }

    fn setup(test_name: &str) -> TestSession {
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

        let mut lua = LuaScripting::new();
        lua.start_session(
            Rc::clone(&scene_manager),
            Rc::clone(&physics_engine),
            Rc::clone(&input_handler),
        )
        .expect("Failed to start Lua session");

        TestSession {
            lua,
            scene_manager,
            physics: physics_engine,
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
            .lua
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
            .lua
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
            .lua
            .load("return script_state.state.first or 0, script_state.state.second or 0")
            .eval()
            .unwrap();
        assert_eq!(first, 1, "first script's update() should have run");
        assert_eq!(second, 1, "second script's update() should have run");
    }

    #[test]
    fn test_collision_query_and_end_game() {
        use rust_2d_game_engine::ecs::PhysicsProperties;

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
                    .get_colliding_entities(&player_id)
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
}
