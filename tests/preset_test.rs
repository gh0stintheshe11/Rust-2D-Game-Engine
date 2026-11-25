//! Attribute presets: they apply cleanly, and the engine reads what they
//! add (a typo in a name or a wrong type would silently do nothing).

use rust_2d_game_engine::physics::{ColliderShape, PhysicsEngine};
use rust_2d_game_engine::render::{RenderEngine, advance_animations, parse_color};
use rust_2d_game_engine::scene::{
    AttributePreset, AttributeValue, Entity, Scene, attribute_presets,
};
use std::collections::HashSet;
use uuid::Uuid;

fn preset(name: &str) -> AttributePreset {
    attribute_presets()
        .into_iter()
        .find(|preset| preset.name == name)
        .unwrap_or_else(|| panic!("no preset {}", name))
}

/// An entity in `scene` at `(x, y)` with the named presets applied.
fn with_presets(scene: &mut Scene, position: (f32, f32), presets: &[&str]) -> Uuid {
    let id = scene.create_entity("thing").unwrap();
    let entity = scene.get_entity_mut(id).unwrap();
    entity.set_position(position.0, position.1, 0.0).unwrap();
    for name in presets {
        entity.apply_preset(&preset(name));
    }
    id
}

#[test]
fn test_presets_apply_once_and_keep_existing_values() {
    let presets = attribute_presets();
    let names: HashSet<&str> = presets.iter().map(|preset| preset.name).collect();
    assert_eq!(names.len(), presets.len(), "preset names are unique");

    for preset in &presets {
        let mut entity = Entity::new(Uuid::new_v4(), "thing").unwrap();
        assert!(!entity.has_preset(preset));
        assert_eq!(
            entity.apply_preset(preset),
            preset.attributes.len(),
            "{}",
            preset.name
        );
        assert!(entity.has_preset(preset));
        for (name, value) in &preset.attributes {
            let attribute = entity.get_attribute_by_name(name).unwrap();
            assert_eq!(&attribute.value, value, "{} in {}", name, preset.name);
        }
        assert_eq!(entity.apply_preset(preset), 0, "{} twice", preset.name);
    }

    // Attributes the entity has keep their values
    let mut entity = Entity::new(Uuid::new_v4(), "thing").unwrap();
    entity
        .set_or_create_attribute("frame", AttributeValue::Integer(5))
        .unwrap();
    assert_eq!(entity.apply_preset(&preset("Sprite sheet")), 1);
    assert_eq!(entity.get_float("frame"), Some(5.0));
}

#[test]
fn test_physics_presets_do_what_they_say() {
    let mut scene = Scene::new("test_scene").unwrap();
    let mut physics = PhysicsEngine::new();
    let floor = with_presets(
        &mut scene,
        (0.0, 200.0),
        &["Static body", "Collider size", "Circle collider"],
    );
    let crate_ = with_presets(
        &mut scene,
        (100.0, 0.0),
        &[
            "Physics body",
            "Collider size",
            "Collision layers",
            "Gravity scale",
        ],
    );
    let zone = with_presets(&mut scene, (300.0, 0.0), &["Trigger zone", "Collider size"]);
    let lift = with_presets(
        &mut scene,
        (400.0, 0.0),
        &["Kinematic body", "Collider size"],
    );
    physics.load_scene(&scene);
    physics.set_velocity(lift, (0.0, -60.0));
    for _ in 0..60 {
        physics.step(&mut scene);
    }

    let y = |id: Uuid| scene.get_entity(id).unwrap().get_y();
    assert_eq!(y(floor), 200.0, "static bodies stay put");
    assert!(y(crate_) > 10.0, "physics bodies fall: y = {}", y(crate_));
    assert!(
        y(lift) < -50.0,
        "kinematic bodies follow their velocity: y = {}",
        y(lift)
    );
    let outlines = physics.collider_outlines();
    let outline = |size: (f32, f32), shape: ColliderShape, sensor: bool| {
        outlines
            .iter()
            .filter(|o| o.size == size && o.shape == shape && o.is_sensor == sensor)
            .count()
    };
    assert_eq!(
        outline((32.0, 32.0), ColliderShape::Circle, false),
        1,
        "the floor"
    );
    assert_eq!(
        outline((32.0, 32.0), ColliderShape::Rectangle, true),
        1,
        "the zone"
    );
    assert!(physics.get_colliding_entities(zone).is_empty());

    // A gravity field pulls a body that has gravity, with no global gravity
    let mut scene = Scene::new("test_scene").unwrap();
    let mut physics = PhysicsEngine::new();
    physics.set_gravity(0.0, 0.0);
    with_presets(&mut scene, (0.0, 0.0), &["Gravity field", "Static body"]);
    let moon = with_presets(&mut scene, (200.0, 0.0), &["Physics body", "Collider size"]);
    physics.load_scene(&scene);
    for _ in 0..30 {
        physics.step(&mut scene);
    }
    assert!(scene.get_entity(moon).unwrap().get_x() < 200.0, "pulled in");
}

#[test]
fn test_sprite_and_animation_presets_are_read() {
    let image = std::env::temp_dir().join(format!("rust2d_preset_{}.png", Uuid::new_v4()));
    // A 64 x 32 sheet: two 32 x 32 frames
    image::RgbaImage::new(64, 32).save(&image).unwrap();
    let mut scene = Scene::new("test_scene").unwrap();
    let id = with_presets(
        &mut scene,
        (0.0, 0.0),
        &[
            "Scale",
            "Rotation",
            "Mirror",
            "Opacity",
            "Tint",
            "Animation",
        ],
    );
    scene.get_entity_mut(id).unwrap().add_image(image).unwrap();

    assert!(
        advance_animations(&mut scene, 0.15).is_empty(),
        "a valid animation"
    );
    let entity = scene.get_entity(id).unwrap();
    assert_eq!(entity.get_float("frame"), Some(1.0), "10 frames per second");
    assert!(parse_color(entity.get_string("tint").unwrap()).is_ok());

    let mut renderer = RenderEngine::new();
    renderer.update_viewport_size(800.0, 600.0);
    let queue = renderer.render(&scene);
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].screen_size, (32.0, 32.0), "one frame of the sheet");
    assert_eq!(queue[0].tint, egui::Color32::WHITE, "white, fully opaque");
    assert_eq!(queue[0].rotation, 0.0);
}
