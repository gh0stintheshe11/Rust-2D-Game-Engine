use rust_2d_game_engine::render::{
    Camera, RenderEngine, camera_frame, game_view, parse_color, sheet_frame,
};
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
    // More than 20 sprites, some with a NaN depth (only possible by writing
    // the attribute directly, e.g. corrupted data): sorting must not panic
    let image = temp_png(2, 2);
    for i in 0..30 {
        let id = sprite(
            &mut scene,
            &format!("s{}", i),
            image.clone(),
            (i as f32, 0.0, i as f32),
        );
        if i % 3 == 0 {
            let entity = scene.get_entity_mut(id).unwrap();
            let z = entity.get_attribute_by_name("z").unwrap().id;
            entity.attributes.get_mut(&z).unwrap().value = AttributeValue::Float(f32::NAN);
        }
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

#[test]
fn test_parse_color() {
    assert_eq!(
        parse_color("#ff8000"),
        Ok(egui::Color32::from_rgb(255, 128, 0))
    );
    assert_eq!(
        parse_color("#FF800080"),
        Ok(egui::Color32::from_rgba_unmultiplied(255, 128, 0, 128))
    );
    for invalid in ["ff8000", "#ff80", "#gg0000", "#ff8000801", "red", "", "#"] {
        assert!(parse_color(invalid).is_err(), "{:?}", invalid);
    }
}

#[test]
fn test_camera_frame() {
    let mut scene = Scene::new("test_scene").unwrap();
    set_camera(&mut scene, (100.0, 50.0), (200.0, 100.0), 2.0);
    let frame = camera_frame(&scene).unwrap();
    // Zoom 2 shows half the camera size of the world, centered on x/y
    assert_eq!(frame.world_min, (50.0, 25.0));
    assert_eq!(frame.world_size, (100.0, 50.0));
    assert_eq!(frame.size, (200.0, 100.0), "text coordinates ignore zoom");

    set_camera(&mut scene, (0.0, 0.0), (0.0, 100.0), 1.0);
    assert!(camera_frame(&scene).is_none(), "a zero-width camera");
}

#[test]
fn test_flip_opacity_and_tint() {
    let mut renderer = RenderEngine::new();
    renderer.update_viewport_size(800.0, 600.0);
    let mut scene = Scene::new("test_scene").unwrap();
    let image = temp_png(4, 4);
    let attributes = |scene: &mut Scene, id: Uuid, values: Vec<(&str, AttributeValue)>| {
        let entity = scene.get_entity_mut(id).unwrap();
        for (name, value) in values {
            entity.set_or_create_attribute(name, value).unwrap();
        }
    };

    let plain = sprite(&mut scene, "plain", image.clone(), (0.0, 0.0, 0.0));
    let styled = sprite(&mut scene, "styled", image.clone(), (10.0, 0.0, 1.0));
    attributes(
        &mut scene,
        styled,
        vec![
            ("flip_x", AttributeValue::Boolean(true)),
            ("opacity", AttributeValue::Float(0.5)),
            ("tint", AttributeValue::String("#ff0000".to_string())),
        ],
    );
    let upside_down = sprite(&mut scene, "upside_down", image.clone(), (20.0, 0.0, 2.0));
    attributes(
        &mut scene,
        upside_down,
        vec![("flip_y", AttributeValue::Boolean(true))],
    );
    let broken = sprite(&mut scene, "broken", image, (30.0, 0.0, 3.0));
    attributes(
        &mut scene,
        broken,
        vec![("tint", AttributeValue::String("red".to_string()))],
    );

    let queue = renderer.render(&scene);
    let entry = |id: Uuid| queue.iter().find(|entry| entry.entity_id == id).unwrap();
    let uv = |min: (f32, f32), max: (f32, f32)| {
        egui::Rect::from_min_max(egui::pos2(min.0, min.1), egui::pos2(max.0, max.1))
    };
    assert_eq!(entry(plain).uv, uv((0.0, 0.0), (1.0, 1.0)));
    assert_eq!(entry(plain).tint, egui::Color32::WHITE);
    assert_eq!(entry(styled).uv, uv((1.0, 0.0), (0.0, 1.0)), "mirrored");
    assert_eq!(entry(styled).tint, egui::Color32::RED.gamma_multiply(0.5));
    assert_eq!(entry(upside_down).uv, uv((0.0, 1.0), (1.0, 0.0)));
    assert_eq!(
        entry(broken).tint,
        egui::Color32::WHITE,
        "an invalid tint is ignored"
    );
}

#[test]
fn test_sprite_sheet_frames() {
    let mut renderer = RenderEngine::new();
    renderer.update_viewport_size(800.0, 600.0);
    let mut scene = Scene::new("test_scene").unwrap();
    // 64 x 32 pixels: a grid of 4 x 2 frames of 16 x 16
    let id = sprite(&mut scene, "runner", temp_png(64, 32), (0.0, 0.0, 0.0));
    let set = |scene: &mut Scene, name: &str, value: AttributeValue| {
        scene
            .get_entity_mut(id)
            .unwrap()
            .set_or_create_attribute(name, value)
            .unwrap();
    };
    set(
        &mut scene,
        "frame_size",
        AttributeValue::Vector2(16.0, 16.0),
    );
    let uv_min = |renderer: &mut RenderEngine, scene: &Scene| {
        let entry = renderer.render(scene).remove(0);
        assert_eq!(entry.screen_size, (16.0, 16.0), "one frame's size");
        (entry.uv.min.x, entry.uv.min.y)
    };

    assert_eq!(
        uv_min(&mut renderer, &scene),
        (0.0, 0.0),
        "frame 0 by default"
    );
    set(&mut scene, "frame", AttributeValue::Integer(5));
    assert_eq!(uv_min(&mut renderer, &scene), (0.25, 0.5), "second row");
    set(&mut scene, "frame", AttributeValue::Integer(9));
    assert_eq!(uv_min(&mut renderer, &scene), (0.25, 0.0), "numbers wrap");
    set(&mut scene, "frame", AttributeValue::Integer(-1));
    assert_eq!(
        uv_min(&mut renderer, &scene),
        (0.75, 0.5),
        "and wrap backwards"
    );

    // Mirroring flips within the frame
    set(&mut scene, "flip_x", AttributeValue::Boolean(true));
    let entry = renderer.render(&scene).remove(0);
    assert_eq!((entry.uv.min.x, entry.uv.max.x), (1.0, 0.75));

    // A frame size larger than the image is the whole image
    let (uv, size) = sheet_frame((10, 10), (64.0, 64.0), 3);
    assert_eq!(
        uv,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
    );
    assert_eq!(size, (10.0, 10.0));
}

#[test]
fn test_sensor_outlines_have_their_own_color() {
    use rust_2d_game_engine::physics::{ColliderOutline, ColliderShape};
    use rust_2d_game_engine::render::{SENSOR_OUTLINE_COLOR, paint_collider_outline};

    // The stroke colors painted for each outline
    let colors = |is_sensor: bool| -> Vec<egui::Color32> {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            for shape in [ColliderShape::Circle, ColliderShape::Rectangle] {
                let outline = ColliderOutline {
                    center: (50.0, 50.0),
                    size: (20.0, 20.0),
                    rotation: 0.0,
                    shape,
                    is_sensor,
                };
                paint_collider_outline(ui.painter(), &outline, egui::Pos2::ZERO);
            }
        });
        // Nothing renders here; textures don't need uploading
        output.textures_delta.clear();
        output
            .shapes
            .into_iter()
            .filter_map(|clipped| match clipped.shape {
                egui::Shape::Circle(circle) => Some(circle.stroke.color),
                egui::Shape::Path(path) => match path.stroke.color {
                    egui::epaint::ColorMode::Solid(color) => Some(color),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    };
    assert_eq!(colors(false), vec![egui::Color32::RED, egui::Color32::BLUE]);
    assert_eq!(colors(true), vec![SENSOR_OUTLINE_COLOR; 2]);
}

#[test]
fn test_tilemaps_draw_their_visible_tiles_as_one_mesh() {
    use rust_2d_game_engine::render::paint_sprites;
    use rust_2d_game_engine::scene::Tilemap;

    let mut renderer = RenderEngine::new();
    renderer.update_viewport_size(800.0, 600.0);
    let mut scene = Scene::new("test_scene").unwrap();
    // A tileset of two 16 x 16 tiles side by side
    let mut map = Tilemap::new(temp_png(32, 16), (16, 16));
    map.set(0, 0, Some(0)).unwrap();
    map.set(1, 0, Some(1)).unwrap();
    map.set(5, 3, Some(0)).unwrap();
    let id = scene.create_entity("ground").unwrap();
    let entity = scene.get_entity_mut(id).unwrap();
    entity.set_position(100.0, 50.0, 0.0).unwrap();
    entity.tilemap = Some(map);

    let queue = renderer.render(&scene);
    assert_eq!(queue.len(), 1, "one entry for the whole map");
    let entry = &queue[0];
    assert_eq!(entry.entity_id, id);
    assert_eq!(entry.screen_pos, (100.0, 50.0));
    assert_eq!(entry.screen_size, (96.0, 64.0), "6 x 4 cells");
    let rects: Vec<egui::Rect> = entry.tiles.iter().map(|tile| tile.rect).collect();
    let rect = |x: f32, y: f32| egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(16.0, 16.0));
    assert_eq!(
        rects,
        vec![rect(100.0, 50.0), rect(116.0, 50.0), rect(180.0, 98.0)]
    );
    // Tile 1 is the tileset's right half, half a texel in on every side
    let uv = entry.tiles[1].uv;
    assert_eq!((uv.min.x, uv.max.x), (0.5 + 0.5 / 32.0, 1.0 - 0.5 / 32.0));
    assert_eq!((uv.min.y, uv.max.y), (0.5 / 16.0, 1.0 - 0.5 / 16.0));

    // Only the cells in the viewport are drawn
    renderer.camera.position = (150.0, 0.0);
    let queue = renderer.render(&scene);
    assert_eq!(
        queue[0].tiles.len(),
        1,
        "the first two cells are left of it"
    );

    // ...all of them in one mesh
    renderer.camera.position = (0.0, 0.0);
    let queue = renderer.render(&scene);
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        paint_sprites(ui.painter(), egui::Pos2::ZERO, &queue, |_| {
            Some(egui::TextureId::User(7))
        });
    });
    output.textures_delta.clear();
    let meshes: Vec<usize> = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Mesh(mesh) => Some(mesh.vertices.len()),
            _ => None,
        })
        .collect();
    assert_eq!(meshes, vec![12], "3 tiles, 4 vertices each");
}
