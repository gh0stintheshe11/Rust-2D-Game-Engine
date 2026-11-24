use rust_2d_game_engine::render::{advance_animations, parse_frames};
use rust_2d_game_engine::scene::{AttributeValue, Scene};
use uuid::Uuid;

#[test]
fn test_parse_frames() {
    assert_eq!(parse_frames("0-3"), Ok(vec![0, 1, 2, 3]));
    assert_eq!(parse_frames("0,1,2,1"), Ok(vec![0, 1, 2, 1]));
    assert_eq!(parse_frames("7-4"), Ok(vec![7, 6, 5, 4]), "backwards");
    assert_eq!(parse_frames(" 2 - 3 , 5 "), Ok(vec![2, 3, 5]));
    assert_eq!(parse_frames("6"), Ok(vec![6]));
    for invalid in ["", "a", "1-", "-1", "1,,2", "1.5", "0-99999"] {
        assert!(parse_frames(invalid).is_err(), "{:?}", invalid);
    }
}

/// A scene with one entity carrying the given attributes.
fn animated(attributes: Vec<(&str, AttributeValue)>) -> (Scene, Uuid) {
    let mut scene = Scene::new("test_scene").unwrap();
    let id = scene.create_entity("runner").unwrap();
    let entity = scene.get_entity_mut(id).unwrap();
    for (name, value) in attributes {
        entity.set_or_create_attribute(name, value).unwrap();
    }
    (scene, id)
}

fn value(scene: &Scene, id: Uuid, name: &str) -> Option<AttributeValue> {
    let entity = scene.get_entity(id).unwrap();
    entity
        .get_attribute_by_name(name)
        .ok()
        .map(|attr| attr.value.clone())
}

fn frame(scene: &Scene, id: Uuid) -> Option<f32> {
    scene.get_entity(id).unwrap().get_float("frame")
}

#[test]
fn test_looping_animation() {
    let (mut scene, id) = animated(vec![(
        "animation",
        AttributeValue::String("0-3".to_string()),
    )]);

    // 10 frames per second by default: 0.25 s in, the third frame shows
    advance_animations(&mut scene, 0.25);
    assert_eq!(frame(&scene, id), Some(2.0));
    assert_eq!(
        value(&scene, id, "frame"),
        Some(AttributeValue::Integer(2)),
        "created as an Integer"
    );

    // The 0.4 s cycle starts over; the stored time wraps with it
    advance_animations(&mut scene, 0.2);
    assert_eq!(frame(&scene, id), Some(0.0));
    let time = scene
        .get_entity(id)
        .unwrap()
        .get_float("animation_time")
        .unwrap();
    assert!((time - 0.05).abs() < 1e-4, "time {}", time);
}

#[test]
fn test_one_shot_animation_stops_on_its_last_frame() {
    let (mut scene, id) = animated(vec![
        ("animation", AttributeValue::String("4,5,6".to_string())),
        ("animation_fps", AttributeValue::Float(20.0)),
        ("animation_loop", AttributeValue::Boolean(false)),
        // Made in the Inspector, which creates Floats by default
        ("frame", AttributeValue::Float(0.0)),
    ]);
    advance_animations(&mut scene, 1.0);
    assert_eq!(
        value(&scene, id, "frame"),
        Some(AttributeValue::Float(6.0)),
        "keeps its type"
    );
    assert_eq!(
        value(&scene, id, "animation_time"),
        Some(AttributeValue::Float(1.0))
    );

    // Resetting the time starts it over (what play_animation does)
    scene
        .get_entity_mut(id)
        .unwrap()
        .set_or_create_attribute("animation_time", AttributeValue::Float(0.0))
        .unwrap();
    advance_animations(&mut scene, 0.01);
    assert_eq!(frame(&scene, id), Some(4.0));
}

#[test]
fn test_invalid_and_missing_animations() {
    let (mut scene, broken) = animated(vec![(
        "animation",
        AttributeValue::String("run".to_string()),
    )]);
    let still = scene.create_entity("still").unwrap();

    let problems = advance_animations(&mut scene, 0.5);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("runner"), "{}", problems[0]);
    assert_eq!(frame(&scene, broken), None, "left unchanged");
    let still = scene.get_entity(still).unwrap();
    assert!(
        still.get_attribute_by_name("animation_time").is_err(),
        "entities without animation get no attributes"
    );
}
