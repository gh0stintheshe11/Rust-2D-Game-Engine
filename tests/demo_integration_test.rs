//! End-to-end tests: load the Star Catcher demo project and play it
//! headlessly through the real game loop (input, scripts, physics,
//! collisions, scene changes).

use rust_2d_game_engine::audio::AudioEngine;
use rust_2d_game_engine::input::InputHandler;
use rust_2d_game_engine::logger::{ConsoleMessageType, LOGGER};
use rust_2d_game_engine::physics::PhysicsEngine;
use rust_2d_game_engine::project::ProjectManager;
use rust_2d_game_engine::render::RenderEngine;
use rust_2d_game_engine::runtime::{GameRuntime, RuntimeState};
use rust_2d_game_engine::save::SaveData;
use rust_2d_game_engine::scene::{AttributeValue, Entity, Scene, SceneManager};
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

fn scene<'a>(manager: &'a SceneManager, name: &str) -> &'a Scene {
    manager
        .get_scene_by_name(name)
        .unwrap_or_else(|| panic!("missing scene {}", name))
}

fn active_scene_name(runtime: &GameRuntime) -> String {
    let manager = runtime.get_scene_manager();
    manager.get_active_scene().unwrap().name.clone()
}

/// The demo, on its title screen, with a fixed random seed and the given
/// share of rocks, and save data in memory.
fn runtime(seed: i32, rock_chance: f32) -> GameRuntime {
    runtime_with_save(seed, rock_chance, SaveData::in_memory())
}

fn runtime_with_save(seed: i32, rock_chance: f32, save_data: SaveData) -> GameRuntime {
    let mut manager = load_demo();
    let main = scene(&manager, "main").id;
    let spawner = manager
        .get_scene_mut(main)
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
    runtime.set_save_data(save_data);
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

/// Two frames through the whole update path (input, tick, paint), the way
/// the player runs the game: one with `key` going down, one with it
/// released.
fn press(runtime: &mut GameRuntime, ctx: &egui::Context, key: egui::Key) {
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(480.0, 640.0));
    for pressed in [true, false] {
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            let ctx = ui.ctx().clone();
            runtime.update(&ctx, ui, viewport);
        });
        // Nothing renders here; textures don't need uploading
        output.textures_delta.clear();
    }
}

/// The texts drawn this frame.
fn texts(runtime: &GameRuntime) -> Vec<String> {
    let texts = runtime.scripting().hud_texts();
    texts.iter().map(|text| text.text.clone()).collect()
}

fn state_number(runtime: &GameRuntime, key: &str) -> f64 {
    runtime
        .scripting()
        .lua()
        .load(format!("return script_state.state.{}", key))
        .eval()
        .unwrap()
}

fn game_over(runtime: &GameRuntime) -> bool {
    runtime
        .scripting()
        .lua()
        .load("return script_state.state.game_over == true")
        .eval()
        .unwrap()
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
    assert_eq!(
        manager.get_active_scene().map(|s| s.name.as_str()),
        Some("title"),
        "the game starts on the title screen"
    );
    for (scene_name, entities) in [
        ("title", &["background", "title"][..]),
        (
            "main",
            &["background", "ground", "paddle", "spawner", "main_camera"][..],
        ),
        ("prefabs", &["star", "rock"][..]),
    ] {
        let scene = scene(&manager, scene_name);
        for name in entities {
            assert!(
                scene.entities.values().any(|e| e.name == *name),
                "missing entity {} in {}",
                name,
                scene_name
            );
        }
    }

    // Every stored path resolves to a file on this machine
    for scene in manager.scenes.values() {
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
    }
}

#[test]
fn test_demo_drops_and_catches_stars() {
    let ctx = egui::Context::default();
    let mut runtime = runtime(42, 0.0);
    play(&mut runtime, 0.5);
    assert_eq!(active_scene_name(&runtime), "title");
    assert!(texts(&runtime).contains(&"Press Space to play".to_string()));
    play(&mut runtime, 3.0);
    assert_eq!(active_scene_name(&runtime), "title", "waits for the player");

    press(&mut runtime, &ctx, egui::Key::Space);
    assert_eq!(active_scene_name(&runtime), "main");
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
        assert!(
            stars
                .iter()
                .all(|s| s.get_string("collides_with") == Some("1")),
            "stars are copies of the prefab"
        );
    }

    play(&mut runtime, 30.0);
    assert!(!game_over(&runtime), "stars never end the round");
    let score = state_number(&runtime, "score");
    assert!(score > 0.0, "stars landing on the paddle are caught");
    assert_eq!(
        texts(&runtime),
        vec![format!("Stars: {}", score)],
        "the score is shown"
    );
    let manager = runtime.get_scene_manager();
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

/// Play until a rock hits the paddle.
fn play_until_game_over(runtime: &mut GameRuntime) {
    for _ in 0..(60.0 / FRAME) as usize {
        runtime.tick(FRAME);
        if game_over(runtime) {
            return;
        }
    }
    panic!("no rock hit the paddle in a minute");
}

#[test]
fn test_a_rock_on_the_paddle_ends_the_round() {
    let ctx = egui::Context::default();
    let mut runtime = runtime(7, 1.0);
    press(&mut runtime, &ctx, egui::Key::Space);
    play_until_game_over(&mut runtime);
    runtime.tick(FRAME);
    assert_eq!(runtime.get_state(), RuntimeState::Playing);
    assert!(texts(&runtime).contains(&"Game over".to_string()));
    assert_eq!(state_number(&runtime, "best"), 0.0, "rocks score nothing");

    // Space plays again: the scene starts over
    press(&mut runtime, &ctx, egui::Key::Space);
    assert_eq!(active_scene_name(&runtime), "main");
    assert!(!game_over(&runtime));
    assert!(entities_named(&runtime.get_scene_manager(), "rock").is_empty());

    // Esc goes back to the title, which shows the best score
    play_until_game_over(&mut runtime);
    press(&mut runtime, &ctx, egui::Key::Escape);
    assert_eq!(active_scene_name(&runtime), "title");
    assert!(texts(&runtime).contains(&"Best: 0".to_string()));

    // Stopping and playing again starts from the title, with a new session
    runtime.restart().unwrap();
    assert_eq!(active_scene_name(&runtime), "title");
    let errors = script_errors();
    assert!(errors.is_empty(), "script errors: {:?}", errors);
}

#[test]
fn test_the_best_score_is_saved_between_runs() {
    let save_file = std::env::temp_dir()
        .join(format!("rust2d_demo_save_{}", uuid::Uuid::new_v4()))
        .join("save.json");
    let ctx = egui::Context::default();

    // A first run of the game: no best score yet
    let mut first = runtime_with_save(7, 1.0, SaveData::open(save_file.clone()));
    first.tick(FRAME);
    assert!(!texts(&first).iter().any(|text| text.starts_with("Best")));
    press(&mut first, &ctx, egui::Key::Space);
    play_until_game_over(&mut first);
    assert!(save_file.is_file(), "the best score was saved");

    // The next run shows it on the title screen
    let mut second = runtime_with_save(7, 1.0, SaveData::open(save_file));
    second.tick(FRAME);
    assert!(texts(&second).contains(&"Best: 0".to_string()));
}
