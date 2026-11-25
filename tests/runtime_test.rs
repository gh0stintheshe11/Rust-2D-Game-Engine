use rust_2d_game_engine::audio::AudioEngine;
use rust_2d_game_engine::input::InputHandler;
use rust_2d_game_engine::physics::PhysicsEngine;
use rust_2d_game_engine::render::RenderEngine;
use rust_2d_game_engine::runtime::{CameraMode, GameRuntime, RuntimeState};
use rust_2d_game_engine::scene::{PhysicsProperties, SceneManager};
use uuid::Uuid;

const FRAME: f32 = 1.0 / 60.0;

fn runtime(scene_manager: SceneManager) -> GameRuntime {
    GameRuntime::new(
        scene_manager,
        PhysicsEngine::new(),
        RenderEngine::new(),
        InputHandler::new(),
        AudioEngine::new(),
        60,
    )
}

/// One scene with a falling box; returns the manager and the box id.
fn falling_box_scene() -> (SceneManager, Uuid) {
    let mut manager = SceneManager::new();
    let scene_id = manager.create_scene("main").unwrap();
    manager.set_active_scene(scene_id).unwrap();
    let props = PhysicsProperties {
        is_movable: true,
        affected_by_gravity: true,
        ..Default::default()
    };
    let box_id = manager
        .get_scene_mut(scene_id)
        .unwrap()
        .create_physical_entity("box", (0.0, 0.0, 0.0), props)
        .unwrap();
    (manager, box_id)
}

fn box_y(runtime: &GameRuntime, box_id: Uuid) -> f32 {
    runtime
        .get_scene_manager()
        .get_active_scene()
        .unwrap()
        .get_entity(box_id)
        .unwrap()
        .get_y()
}

fn with_script(manager: &mut SceneManager, entity: Uuid, source: &str) {
    let path = std::env::temp_dir().join(format!("rust2d_runtime_{}.lua", Uuid::new_v4()));
    std::fs::write(&path, source).unwrap();
    manager
        .get_active_scene_mut()
        .unwrap()
        .get_entity_mut(entity)
        .unwrap()
        .set_script(path)
        .unwrap();
}

#[test]
fn test_play_pause_resume_reset() {
    let (manager, box_id) = falling_box_scene();
    let mut runtime = runtime(manager);
    assert_eq!(runtime.get_state(), RuntimeState::Stopped);

    // Ticking while stopped does nothing
    runtime.tick(FRAME);
    assert_eq!(box_y(&runtime, box_id), 0.0);

    runtime.run().unwrap();
    assert!(runtime.is_playing());
    for _ in 0..30 {
        runtime.tick(FRAME);
    }
    let fallen = box_y(&runtime, box_id);
    assert!(fallen > 1.0, "the box falls while playing: {}", fallen);

    runtime.pause();
    assert!(runtime.is_paused());
    runtime.tick(FRAME);
    assert_eq!(
        box_y(&runtime, box_id),
        fallen,
        "nothing moves while paused"
    );

    runtime.resume();
    runtime.tick(FRAME);
    assert!(box_y(&runtime, box_id) > fallen);

    runtime.reset();
    assert_eq!(runtime.get_state(), RuntimeState::Stopped);
    assert_eq!(box_y(&runtime, box_id), 0.0, "reset restores the scene");
}

#[test]
fn test_run_without_scenes_fails() {
    let mut runtime = runtime(SceneManager::new());
    assert!(runtime.run().is_err());
    assert_eq!(runtime.get_state(), RuntimeState::Stopped);
}

#[test]
fn test_slow_frames_are_capped() {
    // A 1-second stall runs at most 5 physics steps (and the scripts see the
    // same capped time), instead of a burst of catch-up work
    let (mut manager, box_id) = falling_box_scene();
    with_script(
        &mut manager,
        box_id,
        "function update(scene_id, entity_id) script_state.state.t = accumulated_time end",
    );
    let (reference, _) = falling_box_scene();

    let mut stalled = runtime(manager);
    stalled.run().unwrap();
    stalled.tick(1.0);

    let mut smooth = runtime(reference);
    smooth.run().unwrap();
    for _ in 0..5 {
        smooth.tick(FRAME);
    }

    let stalled_y = box_y(&stalled, box_id);
    let smooth_y = smooth
        .get_scene_manager()
        .get_active_scene()
        .unwrap()
        .entities
        .values()
        .find(|e| e.name == "box")
        .unwrap()
        .get_y();
    assert!(
        (stalled_y - smooth_y).abs() < 1e-3,
        "{} vs {}",
        stalled_y,
        smooth_y
    );
}

#[test]
fn test_end_game_and_restart() {
    let (mut manager, box_id) = falling_box_scene();
    with_script(
        &mut manager,
        box_id,
        r#"
        function update(scene_id, entity_id)
            if accumulated_time > 0.1 then end_game() end
        end
        "#,
    );
    let mut runtime = runtime(manager);
    runtime.run().unwrap();

    for _ in 0..20 {
        runtime.tick(FRAME);
    }
    assert_eq!(runtime.get_state(), RuntimeState::Ended);
    let frozen = box_y(&runtime, box_id);
    runtime.tick(FRAME);
    assert_eq!(
        box_y(&runtime, box_id),
        frozen,
        "an ended game stays frozen"
    );

    runtime.restart().unwrap();
    assert!(runtime.is_playing());
    assert_eq!(
        box_y(&runtime, box_id),
        0.0,
        "restart begins from the initial scene"
    );
}

/// Paint one frame headlessly and return the shapes drawn.
fn paint(runtime: &mut GameRuntime) -> Vec<egui::Shape> {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        runtime.paint(ui, viewport);
    });
    // Nothing renders here; textures don't need uploading
    output.textures_delta.clear();
    output
        .shapes
        .into_iter()
        .map(|clipped| clipped.shape)
        .collect()
}

/// Collider outlines are the only circles and paths the runtime draws.
fn is_collider_outline(shape: &egui::Shape) -> bool {
    matches!(shape, egui::Shape::Circle(_) | egui::Shape::Path(_))
}

#[test]
fn test_collider_outlines_are_opt_in() {
    let (manager, _) = falling_box_scene();
    let mut runtime = runtime(manager);
    runtime.run().unwrap();

    assert!(
        !paint(&mut runtime).iter().any(is_collider_outline),
        "exported games must not show debug outlines"
    );
    runtime.set_show_colliders(true);
    assert!(paint(&mut runtime).iter().any(is_collider_outline));
}

/// The texts painted in one frame.
fn painted_texts(runtime: &mut GameRuntime) -> Vec<egui::epaint::TextShape> {
    paint(runtime)
        .into_iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(text) => Some(text),
            _ => None,
        })
        .collect()
}

fn font_size(text: &egui::epaint::TextShape) -> f32 {
    text.galley.job.sections[0].format.font_id.size
}

#[test]
fn test_text_is_laid_out_in_the_camera_frame() {
    // A 400 x 300 camera frame centered on (0, 0), painted into an
    // 800 x 600 viewport: the game, and its text, is shown twice as big
    let (mut manager, box_id) = falling_box_scene();
    {
        let scene = manager.get_active_scene_mut().unwrap();
        let camera_id = scene.default_camera.unwrap();
        let camera = scene.get_entity_mut(camera_id).unwrap();
        camera.set_camera_size(400.0, 300.0).unwrap();
    }
    with_script(
        &mut manager,
        box_id,
        r#"function update(scene_id, entity_id) draw_text("Score: 7", 10, 20) end"#,
    );
    let mut runtime = runtime(manager);
    runtime.run().unwrap();
    runtime.tick(FRAME);

    let texts = painted_texts(&mut runtime);
    assert_eq!(texts.len(), 1);
    assert!(
        (texts[0].pos - egui::pos2(20.0, 40.0)).length() < 0.5,
        "{:?}",
        texts[0].pos
    );
    assert_eq!(font_size(&texts[0]), 48.0);

    // In the editor the game is seen through a free camera; the text stays
    // attached to the camera frame. Here the frame's top-left (-200, -150)
    // is at the viewport's top-left, at zoom 1.
    runtime.set_camera_mode(CameraMode::Free {
        position: (-200.0, -150.0),
        zoom: 1.0,
    });
    let texts = painted_texts(&mut runtime);
    assert!(
        (texts[0].pos - egui::pos2(10.0, 20.0)).length() < 0.5,
        "{:?}",
        texts[0].pos
    );
    assert_eq!(font_size(&texts[0]), 24.0);

    // Pausing keeps the last frame's text on screen
    runtime.pause();
    runtime.tick(FRAME);
    assert_eq!(painted_texts(&mut runtime).len(), 1);
}

#[test]
fn test_animations_advance_only_while_playing() {
    let (mut manager, box_id) = falling_box_scene();
    manager
        .get_active_scene_mut()
        .unwrap()
        .get_entity_mut(box_id)
        .unwrap()
        .set_or_create_attribute(
            "animation",
            rust_2d_game_engine::scene::AttributeValue::String("0-3".to_string()),
        )
        .unwrap();
    let mut runtime = runtime(manager);
    let frame = |runtime: &GameRuntime| {
        runtime
            .get_scene_manager()
            .get_active_scene()
            .unwrap()
            .get_entity(box_id)
            .unwrap()
            .get_float("frame")
    };

    runtime.tick(FRAME);
    assert_eq!(frame(&runtime), None, "stopped: nothing plays");

    runtime.run().unwrap();
    for _ in 0..15 {
        runtime.tick(FRAME);
    }
    assert_eq!(frame(&runtime), Some(2.0), "0.25 s at 10 frames per second");

    runtime.pause();
    for _ in 0..30 {
        runtime.tick(FRAME);
    }
    assert_eq!(frame(&runtime), Some(2.0), "frozen while paused");
}

fn lua_number(runtime: &GameRuntime, expression: &str) -> f64 {
    runtime
        .scripting()
        .lua()
        .load(format!("return {}", expression))
        .eval()
        .unwrap()
}

fn lua_exec(runtime: &GameRuntime, code: &str) {
    runtime.scripting().lua().load(code).exec().unwrap();
}

#[test]
fn test_scripts_switch_scenes() {
    // "main" has a falling box whose script changes scenes on request;
    // "title", where the game starts, switches to main on its first frame
    let (mut manager, box_id) = falling_box_scene();
    let main = manager.active_scene.unwrap();
    with_script(
        &mut manager,
        box_id,
        r#"
        function init(scene_id, entity_id)
            script_state.state.main_inits = (script_state.state.main_inits or 0) + 1
        end
        function update(scene_id, entity_id)
            local go = script_state.state.go
            if go then
                script_state.state.go = nil
                change_scene(go == "here" and scene_id or go)
            end
        end
        "#,
    );
    let title = manager.create_scene("title").unwrap();
    manager.set_active_scene(title).unwrap();
    let logo = manager
        .get_scene_mut(title)
        .unwrap()
        .create_entity("logo")
        .unwrap();
    with_script(
        &mut manager,
        logo,
        r#"
        function init(scene_id, entity_id)
            script_state.state.title_inits = (script_state.state.title_inits or 0) + 1
        end
        function update(scene_id, entity_id)
            draw_text("Press start", 0, 0)
            set_gravity(0, 100)
            change_scene("main")
        end
        "#,
    );
    let mut runtime = runtime(manager);
    assert!(runtime.switch_scene(main).is_err(), "no game is running");
    runtime.run().unwrap();
    assert!(
        runtime.switch_scene(Uuid::new_v4()).is_err(),
        "unknown scene"
    );
    let active = |runtime: &GameRuntime| runtime.get_scene_manager().active_scene;

    // The title's first frame switches to main, which gets its own physics
    // world, keeping the gravity the title set
    runtime.tick(FRAME);
    assert_eq!(active(&runtime), Some(main));
    assert!(
        runtime.scripting().hud_texts().is_empty(),
        "the title's text is gone"
    );
    for _ in 0..30 {
        runtime.tick(FRAME);
    }
    // Half a second of falling: about 12.9 at gravity 100, 6.5 at 50
    let fallen = box_y(&runtime, box_id);
    assert!(fallen > 9.0, "y = {}", fallen);
    assert_eq!(lua_number(&runtime, "script_state.state.title_inits"), 1.0);
    assert_eq!(lua_number(&runtime, "script_state.state.main_inits"), 1.0);

    // Switching to the current scene restarts it
    lua_exec(&runtime, "script_state.state.go = 'here'");
    runtime.tick(FRAME);
    assert_eq!(box_y(&runtime, box_id), 0.0, "back where it started");
    runtime.tick(FRAME);
    assert_eq!(
        lua_number(&runtime, "script_state.state.main_inits"),
        2.0,
        "init runs again"
    );

    // Back to the title, which starts over and switches to main again
    lua_exec(&runtime, "script_state.state.go = 'title'");
    runtime.tick(FRAME);
    assert_eq!(active(&runtime), Some(title));
    runtime.tick(FRAME);
    assert_eq!(lua_number(&runtime, "script_state.state.title_inits"), 2.0);
    assert_eq!(active(&runtime), Some(main));

    // Stopping restores the scenes, including which one is active
    runtime.reset();
    assert_eq!(active(&runtime), Some(title));
    assert_eq!(box_y_in(&runtime, main, box_id), 0.0);
}

fn box_y_in(runtime: &GameRuntime, scene_id: Uuid, box_id: Uuid) -> f32 {
    runtime
        .get_scene_manager()
        .get_scene(scene_id)
        .unwrap()
        .get_entity(box_id)
        .unwrap()
        .get_y()
}

/// Run frames through `GameRuntime::update` in an 800 x 700 window whose
/// top 800 x 600 is the game; `after` adds widgets after the game, the way
/// the editor draws its play buttons over it.
struct Window {
    ctx: egui::Context,
    text: String,
}

const GAME_VIEW: egui::Rect =
    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(800.0, 600.0));

impl Window {
    fn new() -> Self {
        Self {
            ctx: egui::Context::default(),
            text: String::new(),
        }
    }

    fn frame(
        &mut self,
        runtime: &mut GameRuntime,
        events: Vec<egui::Event>,
        mut after: impl FnMut(&mut egui::Ui, &mut String),
    ) {
        let input = egui::RawInput {
            events,
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 700.0),
            )),
            ..Default::default()
        };
        let text = &mut self.text;
        let mut output = self.ctx.run_ui(input, |ui| {
            let ctx = ui.ctx().clone();
            runtime.update(&ctx, ui, GAME_VIEW);
            after(ui, text);
        });
        output.textures_delta.clear();
    }
}

fn no_widgets(_: &mut egui::Ui, _: &mut String) {}

fn pointer_at(x: f32, y: f32) -> egui::Event {
    egui::Event::PointerMoved(egui::pos2(x, y))
}

fn left_button(x: f32, y: f32, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: egui::pos2(x, y),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
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

fn close(actual: (f32, f32), expected: (f32, f32)) -> bool {
    (actual.0 - expected.0).abs() < 0.01 && (actual.1 - expected.1).abs() < 0.01
}

#[test]
fn test_pointer_in_game_coordinates() {
    // A 400 x 300 camera at zoom 2, centered on (0, 0): it shows 200 x 150
    // world units, scaled 4 times into the 800 x 600 game view
    let (mut manager, _) = falling_box_scene();
    {
        let scene = manager.get_active_scene_mut().unwrap();
        let camera_id = scene.default_camera.unwrap();
        let camera = scene.get_entity_mut(camera_id).unwrap();
        camera.set_position(0.0, 0.0, 0.0).unwrap();
        camera.set_camera_size(400.0, 300.0).unwrap();
        camera.set_camera_zoom(2.0).unwrap();
    }
    let mut runtime = runtime(manager);
    runtime.run().unwrap();
    let mut window = Window::new();

    window.frame(&mut runtime, vec![pointer_at(600.0, 450.0)], no_widgets);
    let world = runtime.input().get_mouse_world_pos();
    let screen = runtime.input().get_mouse_screen_pos();
    assert!(close(world, (50.0, 37.5)), "world {:?}", world);
    assert!(
        close(screen, (300.0, 225.0)),
        "three quarters into the 400 x 300 frame: {:?}",
        screen
    );

    // Seen through the editor's free camera instead
    runtime.set_camera_mode(CameraMode::Free {
        position: (10.0, 20.0),
        zoom: 2.0,
    });
    window.frame(&mut runtime, vec![], no_widgets);
    let world = runtime.input().get_mouse_world_pos();
    assert!(close(world, (310.0, 245.0)), "world {:?}", world);
    // Frame pixels follow the world position: (310 + 100, 245 + 75) * 2
    let screen = runtime.input().get_mouse_screen_pos();
    assert!(close(screen, (820.0, 640.0)), "screen {:?}", screen);
}

#[test]
fn test_editor_widgets_keep_their_input() {
    let (manager, _) = falling_box_scene();
    let mut runtime = runtime(manager);
    runtime.run().unwrap();
    let mut window = Window::new();
    let left = egui::PointerButton::Primary;
    // A button over the game's top-left corner, and a text field below it
    let widgets = |focus: bool| {
        move |ui: &mut egui::Ui, text: &mut String| {
            let button = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(100.0, 30.0));
            ui.put(button, egui::Button::new("Pause"));
            let field = egui::Rect::from_min_size(egui::pos2(10.0, 620.0), egui::vec2(200.0, 30.0));
            let response = ui.put(field, egui::TextEdit::singleline(text));
            if focus {
                response.request_focus();
            }
        }
    };

    // Clicking the button: the game doesn't see it
    let mut game_saw_click = false;
    for events in [
        vec![pointer_at(50.0, 25.0)],
        vec![],
        vec![left_button(50.0, 25.0, true)],
        vec![left_button(50.0, 25.0, false)],
    ] {
        window.frame(&mut runtime, events, widgets(false));
        game_saw_click |= runtime.input().is_mouse_button_just_pressed(left);
    }
    assert!(!game_saw_click, "the click was the button's");

    // Clicking the game itself
    window.frame(&mut runtime, vec![pointer_at(400.0, 300.0)], widgets(false));
    window.frame(&mut runtime, vec![], widgets(false));
    window.frame(
        &mut runtime,
        vec![left_button(400.0, 300.0, true)],
        widgets(false),
    );
    assert!(runtime.input().is_mouse_button_just_pressed(left));
    window.frame(
        &mut runtime,
        vec![left_button(400.0, 300.0, false)],
        widgets(false),
    );

    // Typing into the text field: the game doesn't see the keys
    window.frame(&mut runtime, vec![], widgets(true));
    window.frame(&mut runtime, vec![key(egui::Key::A, true)], widgets(false));
    assert!(!runtime.input().is_key_just_pressed(egui::Key::A));
    window.frame(&mut runtime, vec![key(egui::Key::A, false)], widgets(false));

    // Clicking the game takes the keyboard back
    window.frame(
        &mut runtime,
        vec![
            left_button(400.0, 300.0, true),
            left_button(400.0, 300.0, false),
        ],
        widgets(false),
    );
    window.frame(&mut runtime, vec![key(egui::Key::B, true)], widgets(false));
    assert!(runtime.input().is_key_just_pressed(egui::Key::B));
}

#[test]
fn test_particles_play_with_the_game() {
    use rust_2d_game_engine::render::ParticleSettings;

    // The box gives off sparks; after 10 frames it bursts and is removed
    let (mut manager, box_id) = falling_box_scene();
    {
        let scene = manager.get_active_scene_mut().unwrap();
        let entity = scene.get_entity_mut(box_id).unwrap();
        for (name, value) in ParticleSettings::default().attributes() {
            entity.set_or_create_attribute(name, value).unwrap();
        }
    }
    with_script(
        &mut manager,
        box_id,
        r#"
        local frames = 0
        function update(scene_id, entity_id)
            frames = frames + 1
            if frames == 10 then
                emit_particles(scene_id, entity_id, 50)
                remove_entity(scene_id, entity_id)
            end
        end
        "#,
    );
    let mut runtime = runtime(manager);
    let count = |runtime: &GameRuntime| runtime.render_engine().particle_count();
    runtime.tick(FRAME);
    assert_eq!(count(&runtime), 0, "stopped: nothing plays");

    runtime.run().unwrap();
    for _ in 0..9 {
        runtime.tick(FRAME);
    }
    let streamed = count(&runtime);
    assert!(streamed > 0);
    runtime.tick(FRAME);
    assert_eq!(
        count(&runtime),
        streamed + 50,
        "the burst outlives its emitter"
    );

    runtime.pause();
    for _ in 0..30 {
        runtime.tick(FRAME);
    }
    assert_eq!(count(&runtime), streamed + 50, "frozen while paused");
    runtime.resume();
    for _ in 0..90 {
        runtime.tick(FRAME);
    }
    assert_eq!(count(&runtime), 0, "they die");

    // Stopping the game removes them
    runtime.reset();
    runtime.run().unwrap();
    for _ in 0..5 {
        runtime.tick(FRAME);
    }
    assert!(count(&runtime) > 0);
    runtime.reset();
    assert_eq!(count(&runtime), 0);
}

/// A scene seen through a 400 x 300 camera frame filling the 800 x 600
/// game view of [`Window`]: frame pixel (x, y) is at window point (2x, 2y).
fn ui_scene(script: &str) -> GameRuntime {
    let (mut manager, box_id) = falling_box_scene();
    {
        let scene = manager.get_active_scene_mut().unwrap();
        let camera_id = scene.default_camera.unwrap();
        let camera = scene.get_entity_mut(camera_id).unwrap();
        camera.set_position(0.0, 0.0, 0.0).unwrap();
        camera.set_camera_size(400.0, 300.0).unwrap();
        camera.set_camera_zoom(2.0).unwrap();
    }
    with_script(&mut manager, box_id, script);
    let mut runtime = runtime(manager);
    runtime.run().unwrap();
    runtime
}

#[test]
fn test_buttons_click_when_pressed_and_released_on_them() {
    use rust_2d_game_engine::render::HudItem;

    // A panel, and a button at frame pixels (100, 100) to (200, 140): window
    // points (200, 200) to (400, 280)
    let mut runtime = ui_scene(
        r##"
        function update(scene_id, entity_id)
            draw_rect(0, 0, 400, 30, {color = "#00000080", radius = 4})
            if button("Play", 100, 100, 100, 40) then
                script_state.state.clicks = (script_state.state.clicks or 0) + 1
            end
        end
        "##,
    );
    let mut window = Window::new();
    let clicks = |runtime: &GameRuntime| lua_number(runtime, "script_state.state.clicks or 0");
    let background = |runtime: &GameRuntime| match &runtime.scripting().hud()[..] {
        [HudItem::Rect(_), HudItem::Button(button)] => button.background,
        other => panic!("drawn in order: {:?}", other),
    };

    window.frame(&mut runtime, vec![pointer_at(50.0, 50.0)], no_widgets);
    let idle = background(&runtime);
    window.frame(&mut runtime, vec![pointer_at(300.0, 240.0)], no_widgets);
    let hovered = background(&runtime);
    window.frame(
        &mut runtime,
        vec![left_button(300.0, 240.0, true)],
        no_widgets,
    );
    let pressed = background(&runtime);
    assert!(idle != hovered && hovered != pressed && pressed != idle);
    assert_eq!(clicks(&runtime), 0.0, "not before the release");
    window.frame(
        &mut runtime,
        vec![left_button(300.0, 240.0, false)],
        no_widgets,
    );
    assert_eq!(clicks(&runtime), 1.0);
    window.frame(&mut runtime, vec![], no_widgets);
    assert_eq!(clicks(&runtime), 1.0, "once");

    // Pressed elsewhere and released on it: no click
    window.frame(
        &mut runtime,
        vec![left_button(50.0, 50.0, true)],
        no_widgets,
    );
    window.frame(&mut runtime, vec![pointer_at(300.0, 240.0)], no_widgets);
    window.frame(
        &mut runtime,
        vec![left_button(300.0, 240.0, false)],
        no_widgets,
    );
    // Pressed on it and released elsewhere: no click
    window.frame(
        &mut runtime,
        vec![left_button(300.0, 240.0, true)],
        no_widgets,
    );
    window.frame(&mut runtime, vec![pointer_at(50.0, 50.0)], no_widgets);
    window.frame(
        &mut runtime,
        vec![left_button(50.0, 50.0, false)],
        no_widgets,
    );
    assert_eq!(clicks(&runtime), 1.0);
}

#[test]
fn test_ui_drawing_errors_are_reported_to_scripts() {
    let runtime = ui_scene("function update(scene_id, entity_id) end");
    let results: Vec<bool> = runtime
        .scripting()
        .lua()
        .load(
            r##"
            return {
                (pcall(draw_rect, 0, 0, -1, 10)),
                (pcall(draw_rect, 0, 0, 10, 10, {colour = "#ffffff"})),
                (pcall(draw_rect, 0, 0, 10, 10, {color = "red"})),
                (pcall(draw_rect, 0/0, 0, 10, 10)),
                (pcall(draw_image, "a.png", 0, 0, {width = 0})),
                (pcall(button, "Go", 0, 0, 10, 10, {size = "big"})),
                (pcall(draw_rect, 0, 0, 10, 10, {outline = "#ff0000", outline_width = 3})),
                (pcall(draw_image, "a.png", 0, 0, {width = 32, opacity = 0.5})),
            }
            "##,
        )
        .eval()
        .unwrap();
    assert_eq!(
        results,
        vec![false, false, false, false, false, false, true, true]
    );
}
