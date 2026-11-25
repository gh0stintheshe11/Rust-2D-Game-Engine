//! End-to-end tests of the Hilltop Hop demo: load the project and play it
//! through the whole update path (input, scripts, physics, particles, game
//! UI), headless.

use rust_2d_game_engine::audio::AudioEngine;
use rust_2d_game_engine::input::{GamepadAxis, GamepadButton, GamepadEvent, InputHandler};
use rust_2d_game_engine::logger::{ConsoleMessageType, LOGGER};
use rust_2d_game_engine::physics::PhysicsEngine;
use rust_2d_game_engine::project::ProjectManager;
use rust_2d_game_engine::render::RenderEngine;
use rust_2d_game_engine::runtime::GameRuntime;
use rust_2d_game_engine::save::SaveData;
use std::path::PathBuf;
use uuid::Uuid;

const FRAME: f32 = 1.0 / 60.0;
/// Where the player stands at the start: on the ground, column 3
const START: (f32, f32) = (98.0, 373.0);
const TILE: f32 = 32.0;

fn demo_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("demo/hilltop_hop")
}

/// The demo running through `GameRuntime::update` in a 960 x 540 view,
/// with its save data in memory.
struct Game {
    runtime: GameRuntime,
    ctx: egui::Context,
}

impl Game {
    fn new() -> Self {
        let project = ProjectManager::load_project_full(&demo_dir()).expect("the demo loads");
        ProjectManager::set_project_path(demo_dir().to_string_lossy().into_owned());
        let mut runtime = GameRuntime::new(
            project.scene_manager,
            PhysicsEngine::new(),
            RenderEngine::new(),
            InputHandler::new(),
            AudioEngine::new(),
            60,
        );
        runtime.set_save_data(SaveData::in_memory());
        runtime.set_input_actions(project.metadata.input_actions);
        runtime.run().expect("the demo starts");
        let mut game = Self {
            runtime,
            ctx: egui::Context::default(),
        };
        game.frames(2);
        game
    }

    fn frame(&mut self, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            events,
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(960.0, 540.0),
            )),
            ..Default::default()
        };
        let runtime = &mut self.runtime;
        let mut output = self.ctx.run_ui(input, |ui| {
            let ctx = ui.ctx().clone();
            runtime.update(&ctx, ui, ui.max_rect());
        });
        output.textures_delta.clear();
    }

    fn frames(&mut self, count: usize) {
        for _ in 0..count {
            self.frame(Vec::new());
        }
    }

    fn key(key: egui::Key, pressed: bool) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// Press and release a key.
    fn tap(&mut self, key: egui::Key) {
        self.frame(vec![Self::key(key, true)]);
        self.frame(vec![Self::key(key, false)]);
    }

    /// Hold a key down for a while.
    fn hold(&mut self, key: egui::Key, seconds: f32) {
        self.frame(vec![Self::key(key, true)]);
        self.frames((seconds / FRAME) as usize);
        self.frame(vec![Self::key(key, false)]);
    }

    fn scene_name(&self) -> String {
        let manager = self.runtime.get_scene_manager();
        manager.get_active_scene().unwrap().name.clone()
    }

    fn scene_id(&self) -> Uuid {
        self.runtime.get_scene_manager().active_scene.unwrap()
    }

    fn entity(&self, name: &str) -> Uuid {
        let manager = self.runtime.get_scene_manager();
        let scene = manager.get_active_scene().unwrap();
        scene
            .entities
            .values()
            .find(|entity| entity.name == name)
            .unwrap_or_else(|| panic!("no entity named {}", name))
            .id
    }

    fn player(&self) -> (f32, f32) {
        let player = self.entity("player");
        let manager = self.runtime.get_scene_manager();
        let entity = manager
            .get_active_scene()
            .unwrap()
            .get_entity(player)
            .unwrap();
        (entity.get_x(), entity.get_y())
    }

    fn lua<T: mlua::FromLuaMulti>(&self, code: &str) -> T {
        self.runtime.scripting().lua().load(code).eval().unwrap()
    }

    fn state(&self, key: &str) -> f64 {
        self.lua(&format!("return script_state.state.{}", key))
    }

    fn flag(&self, key: &str) -> bool {
        self.lua(&format!("return script_state.state.{} == true", key))
    }

    /// Put the player somewhere, at rest.
    fn teleport(&mut self, x: f32, y: f32) {
        let code = format!(
            "set_position('{}', '{}', {}, {}) set_velocity('{}', 0, 0)",
            self.scene_id(),
            self.entity("player"),
            x,
            y,
            self.entity("player"),
        );
        self.runtime.scripting().lua().load(&code).exec().unwrap();
    }

    fn texts(&self) -> Vec<String> {
        let texts = self.runtime.scripting().hud_texts();
        texts.into_iter().map(|text| text.text).collect()
    }

    /// From the title screen into the level, settled on the ground.
    fn start_level(&mut self) {
        assert_eq!(self.scene_name(), "title");
        self.tap(egui::Key::Space);
        assert_eq!(self.scene_name(), "level");
        self.frames(30);
    }
}

fn close(actual: f32, expected: f32, within: f32) -> bool {
    (actual - expected).abs() <= within
}

fn script_errors() -> Vec<String> {
    LOGGER
        .get_console_messages()
        .into_iter()
        .filter(|message| message.message_type == ConsoleMessageType::Error)
        .map(|message| message.text)
        .collect()
}

#[test]
fn test_every_asset_exists() {
    let project = ProjectManager::load_project_full(&demo_dir()).unwrap();
    for scene in project.scene_manager.scenes.values() {
        for entity in scene.entities.values() {
            let paths = entity
                .images
                .iter()
                .chain(entity.script.iter())
                .chain(entity.tilemap.iter().map(|map| &map.tileset));
            for path in paths {
                assert!(
                    path.is_file(),
                    "{}: missing {}",
                    entity.name,
                    path.display()
                );
            }
        }
    }
    // Animated tiles show tiles the tileset has
    for scene in project.scene_manager.scenes.values() {
        for map in scene
            .entities
            .values()
            .filter_map(|entity| entity.tilemap.as_ref())
        {
            let (width, height) = image::image_dimensions(&map.tileset).unwrap();
            let tiles = (width / map.tile_size.0) * (height / map.tile_size.1);
            for (tile, animation) in map.tile_animations() {
                assert!(
                    animation.frames().iter().all(|frame| *frame < tiles),
                    "tile {}",
                    tile
                );
            }
        }
    }
    for sound in ["jump", "gem", "hurt", "brick", "checkpoint", "win"] {
        let path = demo_dir().join(format!("assets/sounds/{}.wav", sound));
        assert!(path.is_file(), "missing {}", path.display());
    }
}

#[test]
fn test_running_jumping_collecting_and_falling() {
    let mut game = Game::new();
    assert!(game.texts().contains(&"Hilltop Hop".to_string()));
    game.start_level();
    let (x, y) = game.player();
    assert!(
        close(x, START.0, 1.0) && close(y, START.1, 2.0),
        "on the ground: ({}, {})",
        x,
        y
    );
    assert_eq!((game.state("lives"), game.state("gems")), (3.0, 0.0));

    // Running right picks up the first gem, on the ground at column 8
    game.hold(egui::Key::ArrowRight, 0.8);
    let (x, _) = game.player();
    assert!(x > START.0 + 150.0, "ran to x = {}", x);
    assert_eq!(game.state("gems"), 1.0);
    assert!(
        game.runtime.render_engine().particle_count() > 0,
        "sparkles"
    );

    // Jumping leaves the ground, and lands back on it; a tap is a short hop
    game.frames(30);
    game.frame(vec![Game::key(egui::Key::Space, true)]);
    game.frames(10);
    let (_, airborne) = game.player();
    assert!(airborne < START.1 - 60.0, "jumped to y = {}", airborne);
    game.frame(vec![Game::key(egui::Key::Space, false)]);
    game.frames(60);
    assert!(
        close(game.player().1, START.1, 2.0),
        "landed at y = {}",
        game.player().1
    );
    game.tap(egui::Key::Space);
    game.frames(10);
    let (_, hop) = game.player();
    assert!(
        hop > START.1 - 40.0 && hop < START.1 - 5.0,
        "hopped to y = {}",
        hop
    );
    game.frames(60);
    assert!(
        close(game.player().1, START.1, 2.0),
        "landed at y = {}",
        game.player().1
    );

    // Running on falls into the first pit: a heart less, back at the start
    game.hold(egui::Key::ArrowRight, 1.2);
    game.frames(60);
    assert_eq!(game.state("lives"), 2.0);
    let (x, _) = game.player();
    assert!(close(x, START.0, 1.0), "back at the start: x = {}", x);
    assert!(script_errors().is_empty(), "{:?}", script_errors());
}

#[test]
fn test_bricks_break_from_below() {
    let mut game = Game::new();
    game.start_level();
    // Under the brick at column 28, row 9
    game.teleport(28.0 * TILE + 2.0, START.1);
    game.frames(10);
    let brick: Option<u32> = game.lua(&format!(
        "return get_tile('{}', '{}', 28, 9)",
        game.scene_id(),
        game.entity("ground")
    ));
    assert_eq!(brick, Some(3));
    game.hold(egui::Key::Space, 0.4);
    game.frames(40);
    let brick: Option<u32> = game.lua(&format!(
        "return get_tile('{}', '{}', 28, 9)",
        game.scene_id(),
        game.entity("ground")
    ));
    assert_eq!(brick, None, "broken");
    // Through the hole, to the gem above it
    game.hold(egui::Key::Space, 0.4);
    game.frames(40);
    assert_eq!(game.state("gems"), 1.0);
    assert!(script_errors().is_empty(), "{:?}", script_errors());
}

#[test]
fn test_down_drops_through_planks() {
    let mut game = Game::new();
    game.start_level();
    // On the planks at row 10, columns 21 to 25
    game.teleport(23.0 * TILE, 10.0 * TILE - 43.0);
    game.frames(30);
    let (_, on_plank) = game.player();
    assert!(
        close(on_plank, 10.0 * TILE - 42.0, 2.0),
        "on the plank: y = {}",
        on_plank
    );
    game.tap(egui::Key::ArrowDown);
    game.frames(60);
    assert!(
        close(game.player().1, START.1, 2.0),
        "on the ground: y = {}",
        game.player().1
    );
    // Down on solid ground does nothing
    game.tap(egui::Key::ArrowDown);
    game.frames(30);
    assert!(
        close(game.player().1, START.1, 2.0),
        "still on the ground: y = {}",
        game.player().1
    );
    assert!(script_errors().is_empty(), "{:?}", script_errors());
}

#[test]
fn test_reaching_the_flag_wins_and_keeps_the_best_time() {
    let mut game = Game::new();
    game.start_level();
    game.teleport(103.0 * TILE, START.1);
    game.hold(egui::Key::ArrowRight, 1.0);
    assert!(game.flag("won"), "won");
    assert!(game.texts().contains(&"You made it!".to_string()));
    let best: Option<f64> = game.lua("return load_data('best_time')");
    assert!(best.is_some_and(|time| time > 0.0), "{:?}", best);

    // Enter plays again, once the end screen has been up a moment
    game.frames(40);
    game.tap(egui::Key::Enter);
    game.frames(5);
    assert!(!game.flag("won"));
    assert_eq!(game.state("lives"), 3.0);
    let (x, _) = game.player();
    assert!(close(x, START.0, 1.0), "from the start: x = {}", x);
    assert!(script_errors().is_empty(), "{:?}", script_errors());
}

#[test]
fn test_losing_every_heart_ends_the_round() {
    let mut game = Game::new();
    game.start_level();
    game.lua::<()>("script_state.state.lives = 1");
    // Onto the spikes at column 40
    game.teleport(40.0 * TILE + 2.0, START.1 - 40.0);
    game.frames(30);
    assert!(game.flag("game_over"));
    assert!(game.texts().contains(&"Out of hearts".to_string()));
    // Once "Try again" has the focus, the right arrow moves it to "Title"
    game.frames(40);
    game.tap(egui::Key::ArrowRight);
    game.tap(egui::Key::Enter);
    assert_eq!(game.scene_name(), "title");
    assert!(script_errors().is_empty(), "{:?}", script_errors());
}

#[test]
fn test_gamepads_play_too() {
    let mut game = Game::new();
    let press = |button, pressed| {
        let event = if pressed {
            GamepadEvent::ButtonPressed { gamepad: 0, button }
        } else {
            GamepadEvent::ButtonReleased { gamepad: 0, button }
        };
        vec![GamepadEvent::Connected { gamepad: 0 }, event]
    };
    // A starts the game
    game.runtime
        .queue_gamepad_events(press(GamepadButton::South, true));
    game.frames(1);
    game.runtime
        .queue_gamepad_events(press(GamepadButton::South, false));
    game.frames(30);
    assert_eq!(game.scene_name(), "level");

    // The d-pad runs
    game.runtime
        .queue_gamepad_events(press(GamepadButton::DPadRight, true));
    game.frames(30);
    game.runtime
        .queue_gamepad_events(press(GamepadButton::DPadRight, false));
    game.frames(1);
    assert!(game.player().0 > START.0 + 80.0, "x = {}", game.player().0);

    // On a plank, the stick pushed diagonally runs on it; pushed down, it
    // drops through (the down action needs the stick pushed further)
    let stick = |x, y| {
        [(GamepadAxis::LeftStickX, x), (GamepadAxis::LeftStickY, y)]
            .map(|(axis, value)| GamepadEvent::AxisMoved {
                gamepad: 0,
                axis,
                value,
            })
            .to_vec()
    };
    game.teleport(23.0 * TILE, 10.0 * TILE - 43.0);
    game.frames(30);
    game.runtime.queue_gamepad_events(stick(-0.8, 0.6));
    game.frames(3);
    game.runtime.queue_gamepad_events(stick(0.0, 0.0));
    game.frames(30);
    let (_, on_plank) = game.player();
    assert!(
        close(on_plank, 10.0 * TILE - 42.0, 2.0),
        "still on a plank: y = {}",
        on_plank
    );
    game.runtime.queue_gamepad_events(stick(0.0, 1.0));
    game.frames(3);
    game.runtime.queue_gamepad_events(stick(0.0, 0.0));
    game.frames(60);
    assert!(
        game.player().1 > on_plank + 50.0,
        "dropped: y = {}",
        game.player().1
    );
    assert!(script_errors().is_empty(), "{:?}", script_errors());
}
