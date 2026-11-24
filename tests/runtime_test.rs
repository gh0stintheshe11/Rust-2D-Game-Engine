use rust_2d_game_engine::audio::AudioEngine;
use rust_2d_game_engine::input::InputHandler;
use rust_2d_game_engine::physics::PhysicsEngine;
use rust_2d_game_engine::render::RenderEngine;
use rust_2d_game_engine::runtime::{GameRuntime, RuntimeState};
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
