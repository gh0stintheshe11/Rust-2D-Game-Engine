use rust_2d_game_engine::render::{Camera, RenderEngine, game_view};
use rust_2d_game_engine::scene::{AttributeType, AttributeValue, Scene};
use std::path::PathBuf;
use uuid::Uuid;

/// Write a `width` x `height` PNG to a fresh temp file.
fn temp_png(width: u32, height: u32) -> PathBuf {
    let path = std::env::temp_dir().join(format!("rust2d_render_{}.png", Uuid::new_v4()));
    image::RgbaImage::new(width, height).save(&path).unwrap();
    path
}

fn sprite(scene: &mut Scene, name: &str, image: PathBuf, position: (f32, f32, f32)) -> Uuid {
    let id = scene.create_entity(name).unwrap();
    let entity = scene.get_entity_mut(id).unwrap();
    entity.add_image(image).unwrap();
    entity
        .set_position(position.0, position.1, position.2)
        .unwrap();
    id
}

fn set_camera(scene: &mut Scene, center: (f32, f32), size: (f32, f32), zoom: f32) {
    let camera_id = scene.default_camera.unwrap();
    let camera = scene.get_entity_mut(camera_id).unwrap();
    camera.set_x(center.0).unwrap();
    camera.set_y(center.1).unwrap();
    camera.set_camera_size(size.0, size.1).unwrap();
    camera.set_camera_zoom(zoom).unwrap();
}

#[test]
fn test_camera_operations() {
    let mut camera = Camera::new();
    assert_eq!(camera.position, (0.0, 0.0));
    assert_eq!(camera.zoom, 1.0);

    camera.move_by(10.0, 5.0);
    assert_eq!(camera.position, (10.0, 5.0));

    camera.zoom_by(2.0);
    assert_eq!(camera.zoom, 2.0);

    let screen_pos = camera.world_to_screen((15.0, 10.0));
    assert_eq!(screen_pos, ((15.0 - 10.0) * 2.0, (10.0 - 5.0) * 2.0));

    camera.reset();
    assert_eq!(camera.position, (0.0, 0.0));
    assert_eq!(camera.zoom, 1.0);
}

#[test]
fn test_viewport_update() {
    let mut renderer = RenderEngine::new();
    assert_eq!(renderer.get_viewport_size(), (0.0, 0.0));
    renderer.update_viewport_size(800.0, 600.0);
    assert_eq!(renderer.get_viewport_size(), (800.0, 600.0));
}

#[test]
fn test_render_queue_and_texture_cache() {
    let mut renderer = RenderEngine::new();
    renderer.update_viewport_size(800.0, 600.0);
    renderer.camera.zoom = 2.0;

    let mut scene = Scene::new("test_scene").unwrap();
    let front = sprite(&mut scene, "front", temp_png(4, 2), (10.0, 20.0, 1.0));
    let back = sprite(&mut scene, "back", temp_png(8, 8), (0.0, 0.0, -1.0));

    let queue = renderer.render(&scene);
    assert_eq!(queue.len(), 2);
    assert_eq!(queue[0].entity_id, back, "lower z is drawn first");
    assert_eq!(queue[1].entity_id, front);
    assert_eq!(queue[1].screen_pos, (20.0, 40.0));
    assert_eq!(queue[1].screen_size, (8.0, 4.0), "pixel size times zoom");

    assert_eq!(renderer.get_memory_usage(), (4 * 2 + 8 * 8) * 4);
    renderer.clear_cache();
    assert_eq!(renderer.get_memory_usage(), 0);
}

#[test]
fn test_missing_images_and_nan_depth_are_harmless() {
    let mut renderer = RenderEngine::new();
    renderer.update_viewport_size(800.0, 600.0);
    let mut scene = Scene::new("test_scene").unwrap();
    sprite(
        &mut scene,
        "missing",
        PathBuf::from("/nonexistent/sprite.png"),
        (0.0, 0.0, 0.0),
    );
    // More than 20 sprites, some with a NaN depth: sorting must not panic
    let image = temp_png(2, 2);
    for i in 0..30 {
        let z = if i % 3 == 0 { f32::NAN } else { i as f32 };
        sprite(
            &mut scene,
            &format!("s{}", i),
            image.clone(),
            (i as f32, 0.0, z),
        );
    }

    let queue = renderer.render(&scene);
    assert_eq!(queue.len(), 30, "the missing image is skipped");
    // Rendering again doesn't retry or fail
    assert_eq!(renderer.render(&scene).len(), 30);
}

#[test]
fn test_rotation_and_scale_attributes() {
    let mut renderer = RenderEngine::new();
    renderer.update_viewport_size(800.0, 600.0);
    let mut scene = Scene::new("test_scene").unwrap();
    let id = sprite(&mut scene, "s", temp_png(10, 10), (0.0, 0.0, 0.0));
    let entity = scene.get_entity_mut(id).unwrap();
    entity
        .create_attribute(
            "rotation",
            AttributeType::Float,
            AttributeValue::Float(90.0),
        )
        .unwrap();
    entity
        .create_attribute(
            "scale",
            AttributeType::Vector2,
            AttributeValue::Vector2(2.0, 3.0),
        )
        .unwrap();

    let entry = &renderer.render(&scene)[0];
    assert!((entry.rotation - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    assert_eq!(entry.screen_size, (20.0, 30.0));
}

#[test]
fn test_grid_lines_generation() {
    let mut renderer = RenderEngine::new();
    renderer.update_viewport_size(800.0, 600.0);
    assert!(!renderer.get_grid_lines().is_empty());
}

#[test]
fn test_camera_bounds_follow_zoom() {
    let renderer = RenderEngine::new();
    let mut scene = Scene::new("test_scene").unwrap();
    set_camera(&mut scene, (100.0, 50.0), (200.0, 100.0), 2.0);

    let bounds = renderer.get_game_camera_bounds(&scene);
    // Zoom 2 frames half the camera size: 100 x 50 around (100, 50)
    let (top_left, top_right) = bounds[0];
    assert_eq!(top_left, (50.0, 25.0));
    assert_eq!(top_right, (150.0, 25.0));
}

#[test]
fn test_game_view_fits_and_letterboxes_the_camera() {
    let mut scene = Scene::new("test_scene").unwrap();
    set_camera(&mut scene, (200.0, 150.0), (400.0, 300.0), 1.0);

    // Same aspect ratio: scaled 2x, filling the viewport
    let view = game_view(&scene, (800.0, 600.0)).unwrap();
    assert_eq!(view.camera.zoom, 2.0);
    assert_eq!(view.camera.world_to_screen((0.0, 0.0)), (0.0, 0.0));
    assert_eq!(view.frame, (0.0, 0.0, 800.0, 600.0));

    // Wider viewport: bars left and right, frame centered
    let view = game_view(&scene, (1000.0, 600.0)).unwrap();
    assert_eq!(view.camera.zoom, 2.0);
    assert_eq!(view.frame, (100.0, 0.0, 800.0, 600.0));
    assert_eq!(view.camera.world_to_screen((0.0, 0.0)), (100.0, 0.0));
    assert_eq!(view.camera.world_to_screen((400.0, 300.0)), (900.0, 600.0));

    // Degenerate viewport
    assert!(game_view(&scene, (0.0, 600.0)).is_none());
}
