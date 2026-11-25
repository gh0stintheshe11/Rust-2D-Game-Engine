//! Frame-time benchmark: how long the engine spends per frame on scenes of
//! growing size, headless (no window or GPU).
//!
//! ```text
//! cargo bench --bench frame              # every scenario
//! cargo bench --bench frame -- sprites   # scenarios whose name contains "sprites"
//! ```
//!
//! "tick" is `GameRuntime::tick` (scripts, physics, collision hooks,
//! animations); "paint" is `GameRuntime::paint` into an egui frame plus
//! tessellating its shapes into meshes, the CPU side of drawing. At 60 fps a
//! frame has 16.7 ms for both, and for the GPU.

use rust_2d_game_engine::audio::AudioEngine;
use rust_2d_game_engine::input::InputHandler;
use rust_2d_game_engine::physics::PhysicsEngine;
use rust_2d_game_engine::render::RenderEngine;
use rust_2d_game_engine::runtime::{CameraMode, GameRuntime};
use rust_2d_game_engine::scene::{AttributeValue, PhysicsProperties, Scene, SceneManager};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use uuid::Uuid;

const FRAME: f32 = 1.0 / 60.0;
const WARMUP_FRAMES: usize = 150;
const MEASURED_FRAMES: usize = 240;
const SIZES: [usize; 4] = [100, 500, 1000, 2000];
/// The camera frame (and the viewport it's painted into)
const VIEW: (f32, f32) = (1280.0, 720.0);

struct Scenario {
    name: &'static str,
    description: &'static str,
    build: fn(&mut Scene, &Assets, usize),
}

/// Files the scenarios share.
struct Assets {
    image: PathBuf,
    tileset: PathBuf,
    idle_script: PathBuf,
    busy_script: PathBuf,
    bullet_script: PathBuf,
    spawner_script: PathBuf,
}

impl Assets {
    fn create(dir: &Path) -> Self {
        std::fs::create_dir_all(dir).unwrap();
        let image = dir.join("box.png");
        image::RgbaImage::from_pixel(16, 16, image::Rgba([200, 120, 40, 255]))
            .save(&image)
            .unwrap();
        // Two 16 x 16 tiles
        let tileset = dir.join("tiles.png");
        image::RgbaImage::from_fn(32, 16, |x, _| {
            image::Rgba(if x < 16 {
                [90, 160, 60, 255]
            } else {
                [120, 90, 60, 255]
            })
        })
        .save(&tileset)
        .unwrap();
        let idle_script = dir.join("idle.lua");
        std::fs::write(&idle_script, "function update(scene_id, entity_id) end\n").unwrap();
        // Typical per-frame logic: read attributes, steer, write one back
        let busy_script = dir.join("busy.lua");
        std::fs::write(
            &busy_script,
            r#"
function update(scene_id, entity_id)
    local x = get_attribute(scene_id, entity_id, "x")
    local speed = get_attribute(scene_id, entity_id, "speed")
    if x > 1200 then
        set_velocity(entity_id, -speed, 0)
    elseif x < 40 then
        set_velocity(entity_id, speed, 0)
    end
    set_attribute(scene_id, entity_id, "age", get_attribute(scene_id, entity_id, "age") + delta_time)
end
"#,
        )
        .unwrap();
        // Bullets fly right and are removed once off screen
        let bullet_script = dir.join("bullet.lua");
        std::fs::write(
            &bullet_script,
            r#"
function init(scene_id, entity_id)
    set_velocity(entity_id, 600, 0)
end
function update(scene_id, entity_id)
    if get_attribute(scene_id, entity_id, "x") > 1300 then
        remove_entity(scene_id, entity_id)
    end
end
"#,
        )
        .unwrap();
        // Spawns `rate` bullets per frame (an attribute)
        let spawner_script = dir.join("spawner.lua");
        std::fs::write(
            &spawner_script,
            r#"
function update(scene_id, entity_id)
    local rate = get_attribute(scene_id, entity_id, "rate")
    for i = 1, rate do
        spawn(scene_id, "bullet", 0, math.random(20, 680))
    end
end
"#,
        )
        .unwrap();
        Self {
            image,
            tileset,
            idle_script,
            busy_script,
            bullet_script,
            spawner_script,
        }
    }
}

/// Grid positions inside the view for `count` things.
fn grid(index: usize, count: usize) -> (f32, f32) {
    let columns = ((count as f32).sqrt().ceil() as usize).max(1);
    let (column, row) = (index % columns, index / columns);
    let step_x = (VIEW.0 - 80.0) / columns as f32;
    let step_y = (VIEW.1 - 160.0) / columns as f32;
    (40.0 + column as f32 * step_x, 40.0 + row as f32 * step_y)
}

fn sprite(scene: &mut Scene, assets: &Assets, position: (f32, f32)) -> Uuid {
    let id = scene.create_entity("thing").unwrap();
    let entity = scene.get_entity_mut(id).unwrap();
    entity.set_position(position.0, position.1, 0.0).unwrap();
    entity.add_image(assets.image.clone()).unwrap();
    id
}

fn moving_body(scene: &mut Scene, assets: &Assets, position: (f32, f32)) -> Uuid {
    let props = PhysicsProperties {
        is_movable: true,
        affected_by_gravity: false,
        has_collision: false,
        ..Default::default()
    };
    let id = scene
        .create_physical_entity("mover", (position.0, position.1, 0.0), props)
        .unwrap();
    let entity = scene.get_entity_mut(id).unwrap();
    entity.add_image(assets.image.clone()).unwrap();
    for (name, value) in [
        ("speed", AttributeValue::Float(80.0)),
        ("age", AttributeValue::Float(0.0)),
    ] {
        entity.set_or_create_attribute(name, value).unwrap();
    }
    id
}

const SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "sprites",
        description: "static sprites, no scripts or physics",
        build: |scene, assets, count| {
            for i in 0..count {
                sprite(scene, assets, grid(i, count));
            }
        },
    },
    Scenario {
        name: "scripts_idle",
        description: "sprites whose script has an empty update()",
        build: |scene, assets, count| {
            for i in 0..count {
                let id = sprite(scene, assets, grid(i, count));
                let entity = scene.get_entity_mut(id).unwrap();
                entity.set_script(assets.idle_script.clone()).unwrap();
            }
        },
    },
    Scenario {
        name: "scripts_busy",
        description: "moving bodies whose update() reads and writes attributes",
        build: |scene, assets, count| {
            for i in 0..count {
                let id = moving_body(scene, assets, grid(i, count));
                let entity = scene.get_entity_mut(id).unwrap();
                entity.set_script(assets.busy_script.clone()).unwrap();
            }
        },
    },
    Scenario {
        name: "physics_pile",
        description: "falling boxes piling up in a container",
        build: |scene, assets, count| {
            let wall = |scene: &mut Scene, x: f32, y: f32, w: f32, h: f32| {
                let id = scene
                    .create_physical_entity("wall", (x, y, 0.0), PhysicsProperties::default())
                    .unwrap();
                let entity = scene.get_entity_mut(id).unwrap();
                for (name, value) in [("collider_width", w), ("collider_height", h)] {
                    entity
                        .set_or_create_attribute(name, AttributeValue::Float(value))
                        .unwrap();
                }
            };
            wall(scene, 0.0, VIEW.1 - 20.0, VIEW.0, 20.0);
            wall(scene, 0.0, -2000.0, 20.0, 2000.0 + VIEW.1);
            wall(scene, VIEW.0 - 20.0, -2000.0, 20.0, 2000.0 + VIEW.1);
            let props = PhysicsProperties {
                is_movable: true,
                affected_by_gravity: true,
                has_collision: true,
                ..Default::default()
            };
            for i in 0..count {
                let (x, y) = grid(i, count);
                let id = scene
                    .create_physical_entity("box", (x, y - 400.0, 0.0), props.clone())
                    .unwrap();
                scene
                    .get_entity_mut(id)
                    .unwrap()
                    .add_image(assets.image.clone())
                    .unwrap();
            }
        },
    },
    Scenario {
        name: "tilemap",
        description: "a tilemap of 10 tiles per entity, wider than the view, and a box per 10 tiles falling on it",
        build: |scene, assets, count| {
            // 20 rows of 16 px from y = 400 to 720; as many columns as needed
            let rows = 20;
            let columns = (count * 10 / rows).max(1) as i32;
            let mut map =
                rust_2d_game_engine::scene::Tilemap::new(assets.tileset.clone(), (16, 16));
            for row in 0..rows as i32 {
                for column in 0..columns {
                    map.set(column, row, Some(u32::from(row > 0))).unwrap();
                }
            }
            let ground = scene
                .create_physical_entity("ground", (0.0, 400.0, 0.0), PhysicsProperties::default())
                .unwrap();
            scene.get_entity_mut(ground).unwrap().tilemap = Some(Box::new(map));
            let props = PhysicsProperties {
                is_movable: true,
                affected_by_gravity: true,
                has_collision: true,
                ..Default::default()
            };
            let boxes = (count / 10).max(1);
            for i in 0..boxes {
                let (x, y) = grid(i, boxes);
                let id = scene
                    .create_physical_entity("box", (x, y * 0.5, 1.0), props.clone())
                    .unwrap();
                scene
                    .get_entity_mut(id)
                    .unwrap()
                    .add_image(assets.image.clone())
                    .unwrap();
            }
        },
    },
    Scenario {
        name: "particles",
        description: "emitters keeping about five particles per entity alive (10,000 at 2000)",
        build: |scene, _assets, count| {
            let emitters = (count / 200).max(1);
            // Each particle lives a second: the rates add up to the count
            let settings = rust_2d_game_engine::render::ParticleSettings {
                rate: (count * 5) as f32 / emitters as f32,
                lifetime: 1.0,
                variation: 0.0,
                spread: 360.0,
                velocity: (0.0, -60.0),
                gravity: (0.0, 0.0),
                ..Default::default()
            };
            for i in 0..emitters {
                let (x, y) = grid(i, emitters);
                let id = scene.create_entity("emitter").unwrap();
                let entity = scene.get_entity_mut(id).unwrap();
                entity.set_position(x + 40.0, y + 100.0, 0.0).unwrap();
                for (name, value) in settings.attributes() {
                    entity.set_or_create_attribute(name, value).unwrap();
                }
            }
        },
    },
    Scenario {
        name: "spawn_bullets",
        description: "bullets spawned from a template and removed off screen, about this many alive",
        build: |scene, assets, count| {
            // The template flies off and is removed too; spawn() copies it as
            // it was when the game started
            let template = moving_body(scene, assets, (-100.0, -100.0));
            let entity = scene.get_entity_mut(template).unwrap();
            entity.change_entity_name("bullet").unwrap();
            entity.set_script(assets.bullet_script.clone()).unwrap();
            // A bullet lives 1300 / 600 s, about 130 frames
            let rate = (count / 130).max(1) as i32;
            let spawner = scene.create_entity("spawner").unwrap();
            let entity = scene.get_entity_mut(spawner).unwrap();
            entity.set_script(assets.spawner_script.clone()).unwrap();
            entity
                .set_or_create_attribute("rate", AttributeValue::Integer(rate))
                .unwrap();
        },
    },
];

struct Timings {
    tick: Vec<Duration>,
    paint: Vec<Duration>,
}

fn stats(samples: &[Duration]) -> (f64, f64) {
    let ms = |d: &Duration| d.as_secs_f64() * 1000.0;
    let mean = samples.iter().map(ms).sum::<f64>() / samples.len() as f64;
    let max = samples.iter().map(ms).fold(0.0, f64::max);
    (mean, max)
}

fn run(scenario: &Scenario, assets: &Assets, count: usize) -> Timings {
    let mut manager = SceneManager::new();
    let scene_id = manager.create_scene("bench").unwrap();
    manager.set_active_scene(scene_id).unwrap();
    {
        let scene = manager.get_scene_mut(scene_id).unwrap();
        let camera = scene.default_camera.unwrap();
        let camera = scene.get_entity_mut(camera).unwrap();
        camera
            .set_position(VIEW.0 / 2.0, VIEW.1 / 2.0, 0.0)
            .unwrap();
        camera.set_camera_size(VIEW.0, VIEW.1).unwrap();
        (scenario.build)(scene, assets, count);
    }
    let mut runtime = GameRuntime::new(
        manager,
        PhysicsEngine::new(),
        RenderEngine::new(),
        InputHandler::new(),
        AudioEngine::offline(
            std::num::NonZero::new(44_100).unwrap(),
            std::num::NonZero::new(2).unwrap(),
        )
        .0,
        60,
    );
    runtime.set_camera_mode(CameraMode::Game);
    runtime.run().unwrap();

    let ctx = egui::Context::default();
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(VIEW.0, VIEW.1));
    let mut timings = Timings {
        tick: Vec::with_capacity(MEASURED_FRAMES),
        paint: Vec::with_capacity(MEASURED_FRAMES),
    };
    for frame in 0..WARMUP_FRAMES + MEASURED_FRAMES {
        let start = Instant::now();
        runtime.tick(FRAME);
        let tick = start.elapsed();

        let start = Instant::now();
        let input = egui::RawInput {
            screen_rect: Some(viewport),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| runtime.paint(ui, viewport));
        let meshes = ctx.tessellate(std::mem::take(&mut output.shapes), 1.0);
        std::hint::black_box(meshes);
        output.textures_delta.clear();
        let paint = start.elapsed();

        if frame >= WARMUP_FRAMES {
            timings.tick.push(tick);
            timings.paint.push(paint);
        }
    }
    runtime.reset();
    timings
}

fn main() {
    // `cargo bench` passes --bench; any other argument filters scenarios
    let filter: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    let dir = std::env::temp_dir().join(format!("rust2d_bench_{}", Uuid::new_v4()));
    let assets = Assets::create(&dir);

    println!(
        "{:<14} {:>8}  {:>17}  {:>17}",
        "scenario", "entities", "tick ms mean/max", "paint ms mean/max"
    );
    for scenario in SCENARIOS {
        if !filter.is_empty() && !filter.iter().any(|f| scenario.name.contains(f.as_str())) {
            continue;
        }
        for count in SIZES {
            let timings = run(scenario, &assets, count);
            let (tick_mean, tick_max) = stats(&timings.tick);
            let (paint_mean, paint_max) = stats(&timings.paint);
            println!(
                "{:<14} {:>8}  {:>8.3} {:>8.3}  {:>8.3} {:>8.3}",
                scenario.name, count, tick_mean, tick_max, paint_mean, paint_max
            );
        }
        println!("  ({})", scenario.description);
    }
    let _ = std::fs::remove_dir_all(dir);
}
