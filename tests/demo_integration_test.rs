/// End-to-end integration test: loads the real Flappy Bird demo project and
/// simulates several seconds of gameplay headlessly (scripts + physics),
/// asserting the game actually plays: the bird falls, the spawner script
/// creates pipes at runtime, and pipes move left under script control.
#[cfg(test)]
mod tests {
    use rust_2d_game_engine::ecs::AttributeValue;
    use rust_2d_game_engine::input_handler::InputHandler;
    use rust_2d_game_engine::lua_scripting::LuaScripting;
    use rust_2d_game_engine::physics_engine::PhysicsEngine;
    use rust_2d_game_engine::project_manager::ProjectManager;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::rc::Rc;

    const FRAME_DT: f32 = 1.0 / 60.0;

    struct Sim {
        lua: LuaScripting,
        scene_manager: Rc<RefCell<rust_2d_game_engine::ecs::SceneManager>>,
        physics: Rc<RefCell<PhysicsEngine>>,
        input: Rc<RefCell<InputHandler>>,
        scene_id: uuid::Uuid,
    }

    impl Sim {
        fn step_frames(&mut self, frames: usize) {
            for _ in 0..frames {
                self.lua.update_global_time(FRAME_DT).unwrap();
                self.lua.bind_keys_pressed(&self.input.borrow()).unwrap();
                self.lua.run_scripts_for_scene(self.scene_id).unwrap();

                let mut manager = self.scene_manager.borrow_mut();
                let scene = manager.get_active_scene_mut().unwrap();
                let updates = self.physics.borrow_mut().step(scene);
                let filtered: Vec<_> = updates
                    .into_iter()
                    .filter(|(_, _, value)| match value {
                        AttributeValue::Float(v) => !v.is_nan(),
                        AttributeValue::Vector2(x, y) => !x.is_nan() && !y.is_nan(),
                        _ => true,
                    })
                    .collect();
                scene.update_entity_attributes(filtered).unwrap();
            }
        }

        fn entity_x_by_name_prefix(&self, prefix: &str) -> Vec<(String, f32)> {
            let manager = self.scene_manager.borrow();
            let scene = manager.get_active_scene().unwrap();
            scene
                .entities
                .values()
                .filter(|e| e.name.starts_with(prefix))
                .map(|e| (e.name.clone(), e.get_x()))
                .collect()
        }
    }

    fn load_demo() -> Sim {
        let demo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("demo/flappy_bird");
        assert!(demo.exists(), "demo project not found at {:?}", demo);

        // Scripts resolve relative asset paths against the open project
        ProjectManager::set_project_path(demo.to_string_lossy().to_string());

        let loaded = ProjectManager::load_project_full(&demo).expect("demo project should load");
        let scene_manager = Rc::new(RefCell::new(loaded.scene_manager));
        let scene_id = scene_manager
            .borrow()
            .active_scene
            .expect("demo should have an active scene");

        let physics = Rc::new(RefCell::new(PhysicsEngine::new()));
        physics
            .borrow_mut()
            .load_scene(scene_manager.borrow().get_active_scene().unwrap());

        let input = Rc::new(RefCell::new(InputHandler::new()));
        let mut lua = LuaScripting::new();
        lua.start_session(
            Rc::clone(&scene_manager),
            Rc::clone(&physics),
            Rc::clone(&input),
        )
        .expect("Lua session should start");

        Sim {
            lua,
            scene_manager,
            physics,
            input,
            scene_id,
        }
    }

    #[test]
    fn test_demo_loads_with_resolved_asset_paths() {
        let sim = load_demo();
        let manager = sim.scene_manager.borrow();
        let scene = manager.get_active_scene().unwrap();

        assert!(
            scene.entities.values().any(|e| e.name == "bird"),
            "demo scene should contain the bird"
        );

        // Every stored asset/script path must resolve to a real file on this
        // machine (this is the portability fix's ground truth)
        for entity in scene.entities.values() {
            for image in &entity.images {
                assert!(
                    image.is_absolute() && image.exists(),
                    "image path should resolve to an existing file: {:?} (entity {})",
                    image,
                    entity.name
                );
            }
            for sound in &entity.sounds {
                assert!(
                    sound.is_absolute() && sound.exists(),
                    "sound path should resolve to an existing file: {:?} (entity {})",
                    sound,
                    entity.name
                );
            }
            if let Some(script) = &entity.script {
                assert!(
                    script.is_absolute() && script.exists(),
                    "script path should resolve to an existing file: {:?} (entity {})",
                    script,
                    entity.name
                );
            }
        }
    }

    #[test]
    fn test_demo_plays_headlessly() {
        let mut sim = load_demo();

        let initial_bird_y = {
            let manager = sim.scene_manager.borrow();
            let scene = manager.get_active_scene().unwrap();
            scene
                .entities
                .values()
                .find(|e| e.name == "bird")
                .expect("bird entity")
                .get_y()
        };
        let initial_entity_count = sim
            .scene_manager
            .borrow()
            .get_active_scene()
            .unwrap()
            .entities
            .len();

        // ~5.5 simulated seconds: past the spawner's 5s threshold
        sim.step_frames(330);

        // Bird must have fallen (no input; +Y is down)
        let bird_y = {
            let manager = sim.scene_manager.borrow();
            let scene = manager.get_active_scene().unwrap();
            scene
                .entities
                .values()
                .find(|e| e.name == "bird")
                .expect("bird entity")
                .get_y()
        };
        assert!(
            bird_y > initial_bird_y + 5.0,
            "bird should fall under gravity: initial y {}, now {}",
            initial_bird_y,
            bird_y
        );

        // The spawner script must have created a pipe pair at runtime
        let spawned = sim.entity_x_by_name_prefix("top_pipe_");
        assert!(
            spawned.len() >= 2,
            "pipe spawner should have created at least one pipe pair; entities now: {} (was {})",
            sim.scene_manager
                .borrow()
                .get_active_scene()
                .unwrap()
                .entities
                .len(),
            initial_entity_count
        );

        // Spawned pipes are script-driven: they must move left over the next second
        let before = sim.entity_x_by_name_prefix("top_pipe_");
        sim.step_frames(60);
        let after = sim.entity_x_by_name_prefix("top_pipe_");

        let moved_left = before.iter().any(|(name, x_before)| {
            after
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, x_after)| *x_after < *x_before - 20.0)
                .unwrap_or(false)
        });
        assert!(
            moved_left,
            "spawned pipes should move left ~50px/s under script control.\nbefore: {:?}\nafter: {:?}",
            before, after
        );
    }
}
