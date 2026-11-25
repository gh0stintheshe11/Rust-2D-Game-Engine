//! Particles: small, short-lived sprites that emitter entities give off, for
//! sparks, smoke, dust or rain. They're only drawn, never collide, and live
//! in the [`RenderEngine`], not in the scene.
//!
//! An entity with a `particle_rate` attribute is an emitter. Its other
//! `particle_*` attributes ([`ParticleSettings`]) describe what it gives off,
//! and its first image, if it has one, is the particles' texture; the
//! emitter itself isn't drawn.

use super::text::parse_rgba;
use super::{FULL_UV, Quad, RenderEngine, RenderQueueEntry, TextureInfo};
use crate::logger::LOGGER;
use crate::scene::{AttributeValue, Entity, Scene};
use indexmap::IndexMap;
use std::collections::HashSet;
use std::path::PathBuf;
use uuid::Uuid;

/// Most particles alive at once, over all emitters: emitters give off new
/// ones only as old ones die.
pub const MAX_PARTICLES: usize = 20_000;

/// Most particles one burst gives off.
pub const MAX_BURST: u32 = 1_000;

/// Side of the square standing for an emitter in the render queue, in
/// screen points, around its position: the editor draws its marker there,
/// and picks it there.
pub const EMITTER_MARKER_SIZE: f32 = 16.0;

/// Texture id of the built-in particle texture, a soft white dot, for
/// emitters without an image.
pub const PARTICLE_TEXTURE_ID: Uuid = Uuid::from_u128(0x7061_7274_6963_6c65_2d64_6f74_0000_0001);

/// Side of the built-in particle texture, in pixels.
const DOT_SIZE: u32 = 32;

/// Whether an entity gives off particles: it has a `particle_rate`.
pub fn is_particle_emitter(entity: &Entity) -> bool {
    entity.attribute("particle_rate").is_some()
}

/// What an emitter gives off: its `particle_*` attributes.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleSettings {
    /// `particle_rate`: particles per second; 0 gives off only bursts.
    pub rate: f32,
    /// `particle_lifetime`: seconds a particle lives.
    pub lifetime: f32,
    /// `particle_velocity`: a new particle's velocity, px/s (+y is down).
    pub velocity: (f32, f32),
    /// `particle_spread`: degrees a new particle's direction varies by
    /// around `velocity`; 360 sends them every way.
    pub spread: f32,
    /// `particle_variation`: 0 to 1, how much slower and shorter-lived than
    /// `velocity` and `lifetime` a particle may randomly be.
    pub variation: f32,
    /// `particle_gravity`: acceleration of the particles, px/s².
    pub gravity: (f32, f32),
    /// `particle_size`: a new particle's width and height, in world units.
    pub size: f32,
    /// `particle_end_size`: its size when it dies, changing linearly.
    pub end_size: f32,
    /// `particle_color`: a new particle's color, unmultiplied RGBA (written
    /// `#rrggbb` or `#rrggbbaa`). The texture is multiplied by it.
    pub color: [u8; 4],
    /// `particle_end_color`: its color when it dies; an opacity of 00 fades
    /// it out.
    pub end_color: [u8; 4],
    /// `particle_area`: width and height of the rectangle particles start
    /// in, from the emitter's `x`/`y`; (0, 0) is that point.
    pub area: (f32, f32),
}

impl Default for ParticleSettings {
    /// A fountain of warm sparks: the "Particles" preset.
    fn default() -> Self {
        Self {
            rate: 60.0,
            lifetime: 1.2,
            velocity: (0.0, -220.0),
            spread: 50.0,
            variation: 0.5,
            gravity: (0.0, 350.0),
            size: 8.0,
            end_size: 2.0,
            color: [255, 210, 127, 255],
            end_color: [255, 75, 31, 0],
            area: (0.0, 0.0),
        }
    }
}

impl ParticleSettings {
    /// The `particle_*` attributes holding these settings: they make an
    /// entity an emitter.
    pub fn attributes(&self) -> Vec<(&'static str, AttributeValue)> {
        use AttributeValue::{Float, Vector2};
        let hex = |[r, g, b, a]: [u8; 4]| {
            let rgb = format!("#{:02x}{:02x}{:02x}", r, g, b);
            AttributeValue::String(if a == 255 {
                rgb
            } else {
                format!("{}{:02x}", rgb, a)
            })
        };
        vec![
            ("particle_rate", Float(self.rate)),
            ("particle_lifetime", Float(self.lifetime)),
            (
                "particle_velocity",
                Vector2(self.velocity.0, self.velocity.1),
            ),
            ("particle_spread", Float(self.spread)),
            ("particle_variation", Float(self.variation)),
            ("particle_gravity", Vector2(self.gravity.0, self.gravity.1)),
            ("particle_size", Float(self.size)),
            ("particle_end_size", Float(self.end_size)),
            ("particle_color", hex(self.color)),
            ("particle_end_color", hex(self.end_color)),
            ("particle_area", Vector2(self.area.0, self.area.1)),
        ]
    }

    /// An emitter's settings, from its attributes. Missing attributes keep
    /// their default; so do invalid ones, which the returned problems
    /// describe. `None` if the entity isn't an emitter.
    pub fn of(entity: &Entity) -> Option<(Self, Vec<String>)> {
        if !is_particle_emitter(entity) {
            return None;
        }
        let default = Self::default();
        let mut read = Reader {
            entity,
            problems: Vec::new(),
        };
        let settings = Self {
            rate: read.number("particle_rate", default.rate, "at least 0", |v| v >= 0.0),
            lifetime: read.number("particle_lifetime", default.lifetime, "above 0", |v| {
                v > 0.0
            }),
            velocity: read.vector("particle_velocity", default.velocity),
            spread: read
                .number("particle_spread", default.spread, "", |_| true)
                .clamp(0.0, 360.0),
            variation: read
                .number("particle_variation", default.variation, "", |_| true)
                .clamp(0.0, 1.0),
            gravity: read.vector("particle_gravity", default.gravity),
            size: read.number("particle_size", default.size, "at least 0", |v| v >= 0.0),
            end_size: read.number("particle_end_size", default.end_size, "at least 0", |v| {
                v >= 0.0
            }),
            color: read.color("particle_color", default.color),
            end_color: read.color("particle_end_color", default.end_color),
            area: {
                let (width, height) = read.vector("particle_area", default.area);
                (width.abs(), height.abs())
            },
        };
        Some((settings, read.problems))
    }
}

/// Reads an emitter's attributes, noting the invalid ones.
struct Reader<'a> {
    entity: &'a Entity,
    problems: Vec<String>,
}

impl Reader<'_> {
    fn problem(&mut self, name: &str, expected: &str) {
        self.problems.push(format!(
            "Entity '{}': {} must be {}",
            self.entity.name, name, expected
        ));
    }

    fn number(&mut self, name: &str, default: f32, range: &str, valid: fn(f32) -> bool) -> f32 {
        let value = match self.entity.attribute(name).map(|attr| &attr.value) {
            None => return default,
            Some(AttributeValue::Float(value)) => *value,
            Some(AttributeValue::Integer(value)) => *value as f32,
            Some(_) => f32::NAN,
        };
        if value.is_finite() && valid(value) {
            value
        } else {
            let expected = if range.is_empty() {
                "a number".to_string()
            } else {
                format!("a number {}", range)
            };
            self.problem(name, &expected);
            default
        }
    }

    fn vector(&mut self, name: &str, default: (f32, f32)) -> (f32, f32) {
        match self.entity.attribute(name).map(|attr| &attr.value) {
            None => default,
            Some(AttributeValue::Vector2(x, y)) if x.is_finite() && y.is_finite() => (*x, *y),
            Some(_) => {
                self.problem(name, "a Vector2 of finite numbers");
                default
            }
        }
    }

    fn color(&mut self, name: &str, default: [u8; 4]) -> [u8; 4] {
        match self.entity.attribute(name).map(|attr| &attr.value) {
            None => default,
            Some(AttributeValue::String(text)) => parse_rgba(text).unwrap_or_else(|_| {
                self.problem(name, "a color, \"#rrggbb\" or \"#rrggbbaa\"");
                default
            }),
            Some(_) => {
                self.problem(name, "a color, \"#rrggbb\" or \"#rrggbbaa\"");
                default
            }
        }
    }
}

/// A burst of particles from an emitter, e.g. from a script's
/// `emit_particles`. It carries the emitter's settings, so it works even
/// when the emitter is removed in the same frame (an explosion where an
/// enemy was).
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleBurst {
    pub emitter: Uuid,
    pub settings: ParticleSettings,
    /// The emitter's image; `None` for the built-in dot
    pub texture: Option<PathBuf>,
    pub z: f32,
    /// Where the particles start (plus the settings' `area`)
    pub origin: (f32, f32),
    pub count: u32,
}

impl ParticleBurst {
    /// `count` particles (at most [`MAX_BURST`]) from an emitter entity, at
    /// its position, or at `origin`; `None` if the entity isn't an emitter.
    pub fn from_entity(entity: &Entity, count: u32, origin: Option<(f32, f32)>) -> Option<Self> {
        let (settings, _) = ParticleSettings::of(entity)?;
        Some(Self {
            emitter: entity.id,
            settings,
            texture: entity.images.first().cloned(),
            z: entity.get_z(),
            origin: origin.unwrap_or((entity.get_x(), entity.get_y())),
            count: count.min(MAX_BURST),
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct Particle {
    /// The particle's center, in world units
    position: (f32, f32),
    velocity: (f32, f32),
    age: f32,
    lifetime: f32,
}

/// One emitter's particles.
#[derive(Clone, Debug)]
struct Group {
    settings: ParticleSettings,
    texture: Option<PathBuf>,
    z: f32,
    particles: Vec<Particle>,
    /// What the rate owes, below one particle
    pending: f32,
    /// The emitter gave off particles this frame: it exists, and is simulated
    emitting: bool,
}

impl Group {
    fn new(settings: ParticleSettings) -> Self {
        Self {
            settings,
            texture: None,
            z: 0.0,
            particles: Vec::new(),
            pending: 0.0,
            emitting: false,
        }
    }
}

/// The particles alive, by emitter.
#[derive(Clone, Debug)]
pub struct ParticleSystem {
    // Insertion order: emitters keep their draw order among equal z
    groups: IndexMap<Uuid, Group>,
    rng: Rng,
    // Invalid settings already reported
    reported: HashSet<String>,
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self {
            groups: IndexMap::new(),
            rng: Rng(0x2545_f491_4f6c_dd1d),
            reported: HashSet::new(),
        }
    }
}

impl ParticleSystem {
    /// Advance by `dt` seconds: every particle moves and ages (and dies at
    /// the end of its life), the emitters of `scene` for which `emitting`
    /// holds give off particles at their rate, and `bursts` add theirs. The
    /// particles of an emitter that is gone, or no longer emitting, live
    /// out their lives.
    pub fn update(
        &mut self,
        scene: &Scene,
        dt: f32,
        emitting: impl Fn(Uuid) -> bool,
        bursts: Vec<ParticleBurst>,
    ) {
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let mut alive = 0;
        for group in self.groups.values_mut() {
            let gravity = group.settings.gravity;
            group.particles.retain_mut(|particle| {
                particle.age += dt;
                particle.velocity.0 += gravity.0 * dt;
                particle.velocity.1 += gravity.1 * dt;
                particle.position.0 += particle.velocity.0 * dt;
                particle.position.1 += particle.velocity.1 * dt;
                particle.age < particle.lifetime
            });
            group.emitting = false;
            alive += group.particles.len();
        }

        for (id, entity) in &scene.entities {
            if !emitting(*id) {
                continue;
            }
            let Some((settings, problems)) = ParticleSettings::of(entity) else {
                continue;
            };
            for problem in problems {
                if self.reported.insert(problem.clone()) {
                    LOGGER.warning(problem);
                }
            }
            let group = self
                .groups
                .entry(*id)
                .or_insert_with(|| Group::new(settings.clone()));
            group.settings = settings;
            group.texture = entity.images.first().cloned();
            group.z = entity.get_z();
            group.emitting = true;
            group.pending += group.settings.rate * dt;
            let owed = group.pending.floor();
            group.pending -= owed;
            // (A saturating conversion: huge rates are capped)
            let count = (owed as usize).min(MAX_PARTICLES - alive);
            let origin = (entity.get_x(), entity.get_y());
            emit(group, origin, count, &mut self.rng);
            alive += count;
        }

        for burst in bursts {
            let count = (burst.count.min(MAX_BURST) as usize).min(MAX_PARTICLES - alive);
            let group = self
                .groups
                .entry(burst.emitter)
                .or_insert_with(|| Group::new(burst.settings.clone()));
            if !group.emitting {
                // The emitter is gone: the burst knows what it was like
                group.settings = burst.settings;
                group.texture = burst.texture;
                group.z = burst.z;
            }
            emit(group, burst.origin, count, &mut self.rng);
            alive += count;
        }

        self.groups
            .retain(|_, group| group.emitting || !group.particles.is_empty());
    }

    /// How many particles are alive.
    pub fn count(&self) -> usize {
        self.groups
            .values()
            .map(|group| group.particles.len())
            .sum()
    }

    /// Remove every particle.
    pub fn clear(&mut self) {
        self.groups.clear();
    }
}

/// Give off `count` new particles from `origin`.
fn emit(group: &mut Group, origin: (f32, f32), count: usize, rng: &mut Rng) {
    let settings = &group.settings;
    let spread = settings.spread.to_radians();
    group.particles.reserve(count);
    for _ in 0..count {
        let position = (
            origin.0 + rng.unit() * settings.area.0,
            origin.1 + rng.unit() * settings.area.1,
        );
        let (sin, cos) = ((rng.unit() - 0.5) * spread).sin_cos();
        let slower = 1.0 - settings.variation * rng.unit();
        let (x, y) = settings.velocity;
        let velocity = ((x * cos - y * sin) * slower, (x * sin + y * cos) * slower);
        let lifetime = settings.lifetime * (1.0 - settings.variation * rng.unit());
        group.particles.push(Particle {
            position,
            velocity,
            age: 0.0,
            lifetime,
        });
    }
}

/// A small, fast random number generator (SplitMix64): particles need
/// variety, not unpredictability, and the same start looks the same.
#[derive(Clone, Debug)]
struct Rng(u64);

impl Rng {
    /// A number in 0..1.
    fn unit(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        // The top 24 bits: every value is exact as an f32
        (z >> 40) as f32 / (1u64 << 24) as f32
    }
}

impl RenderEngine {
    /// Advance the particles: see [`ParticleSystem::update`].
    pub fn update_particles(
        &mut self,
        scene: &Scene,
        dt: f32,
        emitting: impl Fn(Uuid) -> bool,
        bursts: Vec<ParticleBurst>,
    ) {
        self.particles.update(scene, dt, emitting, bursts);
    }

    /// How many particles are alive.
    pub fn particle_count(&self) -> usize {
        self.particles.count()
    }

    /// Remove every particle, e.g. when the game stops.
    pub fn clear_particles(&mut self) {
        self.particles.clear();
    }

    /// The render queue entry of an emitter in the scene: its marker square
    /// (see [`EMITTER_MARKER_SIZE`]) and its particles, if it has any.
    pub(super) fn emitter_entry(&mut self, id: Uuid, entity: &Entity) -> RenderQueueEntry {
        let image = match self.particles.groups.get(&id) {
            Some(group) => group.texture.clone(),
            None => entity.images.first().cloned(),
        };
        let texture_id = self.particle_texture(image);
        let center = self
            .camera
            .world_to_screen((entity.get_x(), entity.get_y()));
        let half = EMITTER_MARKER_SIZE / 2.0;
        RenderQueueEntry {
            entity_id: id,
            texture_id,
            screen_pos: (center.0 - half, center.1 - half),
            screen_size: (EMITTER_MARKER_SIZE, EMITTER_MARKER_SIZE),
            uv: FULL_UV,
            tint: egui::Color32::WHITE,
            rotation: 0.0,
            z: entity.get_z(),
            quads: Some(self.particle_quads(id)),
        }
    }

    /// The render queue entries of particles whose emitter isn't in
    /// `scene` (it was removed after a burst, say).
    pub(super) fn orphan_particle_entries(&mut self, scene: &Scene) -> Vec<RenderQueueEntry> {
        let orphans: Vec<(Uuid, Option<PathBuf>, f32)> = self
            .particles
            .groups
            .iter()
            .filter(|(id, _)| !scene.entities.contains_key(*id))
            .map(|(id, group)| (*id, group.texture.clone(), group.z))
            .collect();
        orphans
            .into_iter()
            .map(|(id, image, z)| RenderQueueEntry {
                entity_id: id,
                texture_id: self.particle_texture(image),
                screen_pos: (0.0, 0.0),
                screen_size: (0.0, 0.0),
                uv: FULL_UV,
                tint: egui::Color32::WHITE,
                rotation: 0.0,
                z,
                quads: Some(self.particle_quads(id)),
            })
            .collect()
    }

    /// An emitter's particles inside the viewport, oldest first.
    fn particle_quads(&self, id: Uuid) -> Vec<Quad> {
        let Some(group) = self.particles.groups.get(&id) else {
            return Vec::new();
        };
        let settings = &group.settings;
        let zoom = self.camera.zoom;
        let viewport = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(self.viewport_size.0, self.viewport_size.1),
        );
        group
            .particles
            .iter()
            .filter_map(|particle| {
                let t = (particle.age / particle.lifetime).clamp(0.0, 1.0);
                let size = (settings.size + (settings.end_size - settings.size) * t) * zoom;
                if size.is_nan() || size <= 0.0 {
                    return None;
                }
                let center = self.camera.world_to_screen(particle.position);
                let rect = egui::Rect::from_center_size(
                    egui::pos2(center.0, center.1),
                    egui::vec2(size, size),
                );
                viewport.intersects(rect).then(|| Quad {
                    rect,
                    uv: FULL_UV,
                    color: blend(settings.color, settings.end_color, t),
                })
            })
            .collect()
    }

    /// The texture of an emitter's particles: its image, or the built-in dot
    /// (also when the image can't be loaded).
    fn particle_texture(&mut self, image: Option<PathBuf>) -> Uuid {
        if let Some(texture) = image.and_then(|path| self.load_texture(&path)) {
            return texture;
        }
        self.texture_cache
            .entry(PARTICLE_TEXTURE_ID)
            .or_insert_with(dot_texture);
        PARTICLE_TEXTURE_ID
    }
}

/// The color `t` (0 to 1) of the way from `from` to `to`, blended before
/// multiplying by opacity, so fading out to a transparent color keeps the
/// hue changing.
fn blend(from: [u8; 4], to: [u8; 4], t: f32) -> egui::Color32 {
    let channel = |i: usize| {
        let (a, b) = (from[i] as f32, to[i] as f32);
        (a + (b - a) * t).round().clamp(0.0, 255.0) as u8
    };
    egui::Color32::from_rgba_unmultiplied(channel(0), channel(1), channel(2), channel(3))
}

/// The built-in particle texture: a white dot, solid in the middle and
/// fading out towards its edge.
fn dot_texture() -> TextureInfo {
    let radius = DOT_SIZE as f32 / 2.0;
    let mut data = Vec::with_capacity((DOT_SIZE * DOT_SIZE * 4) as usize);
    for y in 0..DOT_SIZE {
        for x in 0..DOT_SIZE {
            let dx = (x as f32 + 0.5 - radius) / radius;
            let dy = (y as f32 + 0.5 - radius) / radius;
            let distance = (dx * dx + dy * dy).sqrt();
            // Solid up to half the radius, then a smooth fade
            let t = ((1.0 - distance) * 2.0).clamp(0.0, 1.0);
            let alpha = t * t * (3.0 - 2.0 * t);
            data.extend([255, 255, 255, (alpha * 255.0).round() as u8]);
        }
    }
    TextureInfo {
        data,
        dimensions: (DOT_SIZE, DOT_SIZE),
    }
}
