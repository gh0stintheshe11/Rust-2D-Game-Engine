//! End-to-end tests: load the Star Catcher demo project and play it
//! headlessly through the real game loop (scripts, physics, collisions).

use rust_2d_game_engine::audio::AudioEngine;
use rust_2d_game_engine::input::InputHandler;
use rust_2d_game_engine::logger::{ConsoleMessageType, LOGGER};
use rust_2d_game_engine::physics::PhysicsEngine;
use rust_2d_game_engine::project::ProjectManager;
use rust_2d_game_engine::render::RenderEngine;
use rust_2d_game_engine::runtime::{GameRuntime, RuntimeState};
use rust_2d_game_engine::scene::{AttributeValue, Entity, SceneManager};
use std::path::PathBuf;

const FRAME: f32 = 1.0 / 60.0;

fn demo_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("demo/star_catcher")
}

fn load_demo() -> SceneManager {
    ProjectManager::load_project_full(&demo_dir())
        .expect("the demo project loads")
        .scene_manager
}

/// The demo with a fixed random seed and the given share of rocks.
fn runtime(seed: i32, rock_chance: f32) -> GameRuntime {
    let mut manager = load_demo();
    let spawner = manager
        .get_active_scene_mut()
        .unwrap()
        .entities
        .values_mut()
        .find(|e| e.name == "spawner")
        .expect("spawner entity");
    set(spawner, "seed", AttributeValue::Integer(seed));
    set(spawner, "rock_chance", AttributeValue::Float(rock_chance));

    let mut runtime = GameRuntime::new(
        manager,
        PhysicsEngine::new(),
        RenderEngine::new(),
        InputHandler::new(),
        AudioEngine::new(),
        60,
    );
    runtime.run().expect("the demo starts");
    runtime
}

fn set(entity: &mut Entity, name: &str, value: AttributeValue) {
    let id = entity.get_attribute_by_name(name).unwrap().id;
    entity
        .modify_attribute(id, None, None, Some(value))
        .unwrap();
}

fn play(runtime: &mut GameRuntime, seconds: f32) {
    for _ in 0..(seconds / FRAME) as usize {
        runtime.tick(FRAME);
    }
}

fn entities_named<'a>(manager: &'a SceneManager, name: &str) -> Vec<&'a Entity> {
    manager
        .get_active_scene()
        .unwrap()
        .entities
        .values()
        .filter(|e| e.name == name)
        .collect()
}

fn position(manager: &SceneManager, name: &str) -> (f32, f32) {
    let entity = entities_named(manager, name)[0];
    (entity.get_x(), entity.get_y())
}

fn script_errors() -> Vec<String> {
    LOGGER
        .get_console_messages()
        .into_iter()
        .filter(|m| m.message_type == ConsoleMessageType::Error)
        .map(|m| m.text)
        .collect()
}

#[test]
fn test_demo_assets_resolve() {
    let manager = load_demo();
    let scene = manager.get_active_scene().expect("an active scene");
    for name in ["background", "ground", "paddle", "spawner", "main_camera"] {
        assert!(
            scene.entities.values().any(|e| e.name == name),
            "missing entity {}",
            name
        );
    }

    // Every stored path resolves to a file on this machine
    for entity in scene.entities.values() {
        let paths = entity
            .images
            .iter()
            .chain(&entity.sounds)
            .chain(entity.script.iter());
        for path in paths {
            assert!(
                path.is_absolute() && path.is_file(),
                "{:?} of {} doesn't resolve",
                path,
                entity.name
            );
        }
    }
    for file in ["star.png", "rock.png", "tally.png"] {
        assert!(demo_dir().join("assets/images").join(file).is_file());
    }
}

#[test]
fn test_demo_drops_and_catches_stars() {
    let mut runtime = runtime(42, 0.0);
    let paddle_start = position(&runtime.get_scene_manager(), "paddle");
    let ground_start = position(&runtime.get_scene_manager(), "ground");

    play(&mut runtime, 2.5);
    {
        let manager = runtime.get_scene_manager();
        let stars = entities_named(&manager, "star");
        assert!(!stars.is_empty(), "the spawner drops stars");
        assert!(
            stars.iter().any(|s| s.get_y() > 0.0),
            "stars fall into the field"
        );
        assert!(
            stars
                .iter()
                .any(|s| s.get_attribute_by_name("rotation").is_ok()),
            "falling stars spin"
        );
    }

    play(&mut runtime, 30.0);
    assert_eq!(
        runtime.get_state(),
        RuntimeState::Playing,
        "stars never end the game"
    );
    let manager = runtime.get_scene_manager();
    assert!(
        !entities_named(&manager, "tally").is_empty(),
        "stars landing on the paddle are caught"
    );
    assert!(
        entities_named(&manager, "star").len() < 15,
        "missed stars are removed when they hit the ground"
    );
    assert_eq!(
        position(&manager, "paddle"),
        paddle_start,
        "no input, no movement"
    );
    assert_eq!(
        position(&manager, "ground"),
        ground_start,
        "the ground is fixed"
    );
    let errors = script_errors();
    assert!(errors.is_empty(), "script errors: {:?}", errors);
}

#[test]
fn test_a_rock_on_the_paddle_ends_the_game() {
    let mut runtime = runtime(7, 1.0);
    for _ in 0..(60.0 / FRAME) as usize {
        runtime.tick(FRAME);
        if runtime.get_state() == RuntimeState::Ended {
            break;
        }
    }
    assert_eq!(runtime.get_state(), RuntimeState::Ended);

    // Restarting begins a new game from the saved scene
    runtime.restart().unwrap();
    assert_eq!(runtime.get_state(), RuntimeState::Playing);
    assert!(entities_named(&runtime.get_scene_manager(), "rock").is_empty());
}
