//! Particles: emitters, bursts, and how they're drawn.

use rust_2d_game_engine::render::{
    MAX_BURST, MAX_PARTICLES, PARTICLE_TEXTURE_ID, ParticleBurst, ParticleSettings, RenderEngine,
    is_particle_emitter,
};
use rust_2d_game_engine::scene::{AttributeValue, Scene};
use uuid::Uuid;

const FRAME: f32 = 1.0 / 60.0;

/// An emitter at `position`: the default settings, changed by `attributes`.
fn emitter(scene: &mut Scene, position: (f32, f32), attributes: &[(&str, AttributeValue)]) -> Uuid {
    let id = scene.create_entity("emitter").unwrap();
    let entity = scene.get_entity_mut(id).unwrap();
    entity.set_position(position.0, position.1, 0.0).unwrap();
    // The given attributes first: they may have another type than the default
    for (name, value) in attributes {
        entity.set_or_create_attribute(name, value.clone()).unwrap();
    }
    for (name, value) in ParticleSettings::default().attributes() {
        if !attributes.iter().any(|(given, _)| *given == name) {
            entity.set_or_create_attribute(name, value).unwrap();
        }
    }
    id
}

fn renderer() -> RenderEngine {
    let mut renderer = RenderEngine::new();
    renderer.update_viewport_size(800.0, 600.0);
    renderer
}

fn set(scene: &mut Scene, id: Uuid, name: &str, value: AttributeValue) {
    let entity = scene.get_entity_mut(id).unwrap();
    entity.set_or_create_attribute(name, value).unwrap();
}

/// An 8 x 8 PNG in a temp file, removed when the returned path is dropped.
fn temp_png() -> tempfile::TempPath {
    let path = tempfile::Builder::new()
        .prefix("rust2d_particles_")
        .suffix(".png")
        .tempfile()
        .unwrap()
        .into_temp_path();
    image::RgbaImage::new(8, 8).save(&path).unwrap();
    path
}

#[test]
fn test_emitters_give_off_particles_at_their_rate() {
    use AttributeValue::Float;
    let mut scene = Scene::new("test_scene").unwrap();
    let id = emitter(
        &mut scene,
        (100.0, 100.0),
        &[
            ("particle_rate", Float(60.0)),
            ("particle_lifetime", Float(0.5)),
            ("particle_variation", Float(0.0)),
        ],
    );
    let mut renderer = renderer();
    let run = |renderer: &mut RenderEngine, scene: &Scene, frames: usize| {
        for _ in 0..frames {
            renderer.update_particles(scene, FRAME, |_| true, Vec::new());
        }
        renderer.particle_count()
    };
    // One per frame, each living 30 frames
    assert!((29..=31).contains(&run(&mut renderer, &scene, 30)));
    // As many die as are born
    assert!((29..=31).contains(&run(&mut renderer, &scene, 60)));
    // A rate of 0 lets them die out
    set(&mut scene, id, "particle_rate", Float(0.0));
    assert_eq!(run(&mut renderer, &scene, 31), 0);
}

#[test]
fn test_particles_move_shrink_and_fade() {
    use AttributeValue::{Float, String, Vector2};
    let mut scene = Scene::new("test_scene").unwrap();
    let id = emitter(
        &mut scene,
        (100.0, 100.0),
        &[
            ("particle_rate", Float(0.0)),
            ("particle_lifetime", Float(1.0)),
            ("particle_velocity", Vector2(0.0, -100.0)),
            ("particle_spread", Float(0.0)),
            ("particle_variation", Float(0.0)),
            ("particle_gravity", Vector2(40.0, 0.0)),
            ("particle_size", Float(10.0)),
            ("particle_end_size", Float(0.0)),
            ("particle_color", String("#ffffff".into())),
            ("particle_end_color", String("#ff000000".into())),
        ],
    );
    let mut renderer = renderer();
    let burst = ParticleBurst::from_entity(scene.get_entity(id).unwrap(), 1, None).unwrap();
    renderer.update_particles(&scene, 0.0, |_| true, vec![burst]);
    for _ in 0..30 {
        renderer.update_particles(&scene, FRAME, |_| true, Vec::new());
    }

    // Half a second later: 50 px up, pushed right by gravity, half the
    // size, halfway to transparent red
    let queue = renderer.render(&scene);
    let quads = queue[0].quads.as_ref().expect("an emitter draws quads");
    assert_eq!(quads.len(), 1);
    let (center, size) = (quads[0].rect.center(), quads[0].rect.width());
    assert!((center.y - 50.0).abs() < 1.0, "y = {}", center.y);
    assert!(center.x > 104.0 && center.x < 106.0, "x = {}", center.x);
    assert!((size - 5.0).abs() < 0.1, "size = {}", size);
    let [r, g, _, a] = quads[0].color.to_srgba_unmultiplied();
    assert!(
        r == 255 && (126..=129).contains(&g),
        "red stays, green fades: {:?}",
        quads[0].color
    );
    assert!((126..=129).contains(&a), "alpha = {}", a);
}

#[test]
fn test_bursts_outlive_their_emitter() {
    let mut scene = Scene::new("test_scene").unwrap();
    let id = emitter(
        &mut scene,
        (100.0, 100.0),
        &[("particle_rate", AttributeValue::Float(0.0))],
    );
    let mut renderer = renderer();
    // An enemy explodes and is removed in the same frame
    let entity = scene.get_entity(id).unwrap();
    let burst = ParticleBurst::from_entity(entity, 25, Some((300.0, 200.0))).unwrap();
    assert_eq!(burst.origin, (300.0, 200.0));
    scene.delete_entity(id).unwrap();
    renderer.update_particles(&scene, FRAME, |_| true, vec![burst]);
    assert_eq!(renderer.particle_count(), 25);

    let queue = renderer.render(&scene);
    assert_eq!(queue.len(), 1, "drawn without their emitter");
    assert_eq!(queue[0].quads.as_ref().unwrap().len(), 25);
    assert_eq!(queue[0].texture_id, PARTICLE_TEXTURE_ID, "the built-in dot");

    // ...until they die (the default lifetime is 1.2 s)
    for _ in 0..90 {
        renderer.update_particles(&scene, FRAME, |_| true, Vec::new());
    }
    assert_eq!(renderer.particle_count(), 0);
    assert!(renderer.render(&scene).is_empty());
}

#[test]
fn test_particles_are_limited() {
    let mut scene = Scene::new("test_scene").unwrap();
    let id = emitter(
        &mut scene,
        (0.0, 0.0),
        &[("particle_rate", AttributeValue::Float(1e12))],
    );
    let mut renderer = renderer();
    renderer.update_particles(&scene, FRAME, |_| true, Vec::new());
    assert_eq!(renderer.particle_count(), MAX_PARTICLES);
    let entity = scene.get_entity(id).unwrap();
    let burst = ParticleBurst::from_entity(entity, u32::MAX, None).unwrap();
    assert_eq!(burst.count, MAX_BURST);
    renderer.update_particles(&scene, FRAME, |_| true, vec![burst]);
    assert!(renderer.particle_count() <= MAX_PARTICLES);
}

#[test]
fn test_emitters_draw_a_marker_and_their_particles_not_a_sprite() {
    let mut scene = Scene::new("test_scene").unwrap();
    let plain = scene.create_entity("plain").unwrap();
    assert!(!is_particle_emitter(scene.get_entity(plain).unwrap()));
    let id = emitter(
        &mut scene,
        (100.0, 50.0),
        &[("particle_rate", AttributeValue::Float(0.0))],
    );
    assert!(is_particle_emitter(scene.get_entity(id).unwrap()));
    let image = temp_png();
    scene
        .get_entity_mut(id)
        .unwrap()
        .add_image(image.to_path_buf())
        .unwrap();
    let mut renderer = renderer();

    // Without particles: just the marker square around its position
    let queue = renderer.render(&scene);
    assert_eq!(queue.len(), 1);
    let entry = &queue[0];
    assert_eq!(
        (entry.screen_pos, entry.screen_size),
        ((92.0, 42.0), (16.0, 16.0))
    );
    assert_eq!(
        entry.quads,
        Some(Vec::new()),
        "the image isn't drawn as a sprite"
    );
    // ...whose image is the particles' texture
    assert_eq!(entry.texture_id, RenderEngine::path_to_uuid(&image));

    // Particles outside the viewport aren't drawn
    let burst = ParticleBurst::from_entity(scene.get_entity(id).unwrap(), 10, Some((-500.0, 0.0)));
    renderer.update_particles(&scene, 0.0, |_| true, vec![burst.unwrap()]);
    assert_eq!(renderer.particle_count(), 10);
    assert_eq!(renderer.render(&scene)[0].quads, Some(Vec::new()));
}

#[test]
fn test_only_the_chosen_emitters_emit() {
    let mut scene = Scene::new("test_scene").unwrap();
    let chosen = emitter(&mut scene, (100.0, 100.0), &[]);
    let other = emitter(&mut scene, (300.0, 100.0), &[]);
    let mut renderer = renderer();
    for _ in 0..30 {
        renderer.update_particles(&scene, FRAME, |id| id == chosen, Vec::new());
    }
    let queue = renderer.render(&scene);
    let drawn = |id: Uuid| {
        let entry = queue.iter().find(|entry| entry.entity_id == id).unwrap();
        entry.quads.as_ref().unwrap().len()
    };
    assert!(drawn(chosen) > 0);
    assert_eq!(drawn(other), 0);
}

#[test]
fn test_invalid_settings_keep_their_default() {
    use AttributeValue::{Float, String};
    let mut scene = Scene::new("test_scene").unwrap();
    let id = emitter(
        &mut scene,
        (0.0, 0.0),
        &[
            ("particle_lifetime", Float(-1.0)),
            ("particle_color", String("red".into())),
            ("particle_velocity", Float(3.0)),
            ("particle_spread", Float(720.0)),
        ],
    );
    let (settings, problems) = ParticleSettings::of(scene.get_entity(id).unwrap()).unwrap();
    let default = ParticleSettings::default();
    assert_eq!(
        (settings.lifetime, settings.color, settings.velocity),
        (default.lifetime, default.color, default.velocity)
    );
    assert_eq!(settings.spread, 360.0, "clamped, not a problem");
    assert_eq!(problems.len(), 3, "{:?}", problems);

    // (Attributes can't be NaN, but a frame time could be)
    let mut renderer = renderer();
    renderer.update_particles(&scene, f32::NAN, |_| true, Vec::new());
    assert_eq!(renderer.particle_count(), 0, "a NaN frame time is no time");
}
