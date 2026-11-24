//! 2D physics: a rapier2d world mirroring the scene's physics entities.
//!
//! Coordinates are screen-space pixels with +Y pointing down. An entity's
//! `x`/`y` attributes are the top-left corner of its sprite; its rigid body
//! is centered on the sprite (or on the explicit collider size), so bodies
//! rotate around the sprite's center like the renderer does.

use crate::logger::LOGGER;
use crate::scene::{AttributeType, AttributeValue, Entity, Scene};
use rapier2d::prelude::*;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Global gravity of a new world (and after [`PhysicsEngine::cleanup`]).
pub const DEFAULT_GRAVITY: (f32, f32) = (0.0, 50.0);

/// Default fixed simulation step, in seconds.
pub const DEFAULT_TIME_STEP: f32 = 1.0 / 60.0;

/// Default `gravity_strength` of a `creates_gravity` entity: the attraction
/// it exerts is `strength / distance²` (px/s²), i.e. 50 px/s² at 100 px.
pub const DEFAULT_GRAVITY_FIELD_STRENGTH: f32 = 500_000.0;

/// Gravity fields treat closer bodies as being this far away, so the
/// attraction stays finite when two bodies overlap.
const MIN_GRAVITY_FIELD_DISTANCE: f32 = 10.0;

/// Where a ray first hits a collider (see [`PhysicsEngine::raycast`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    pub entity_id: Uuid,
    /// World position of the hit.
    pub point: (f32, f32),
    /// Distance from the ray's start to the hit.
    pub distance: f32,
}

/// Shape of a collider, for debug drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColliderShape {
    Circle,
    Rectangle,
}

/// World-space outline of one collider, for debug drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColliderOutline {
    pub center: (f32, f32),
    /// Width and height; circles report their diameter for both.
    pub size: (f32, f32),
    /// Rotation around `center`, in radians.
    pub rotation: f32,
    pub shape: ColliderShape,
    /// A trigger zone (`is_sensor`), which reports overlaps without blocking.
    pub is_sensor: bool,
}

/// Bookkeeping for one entity mirrored into the physics world.
struct BodyInfo {
    body: RigidBodyHandle,
    collider: Option<ColliderHandle>,
    /// Offset from the entity's `x`/`y` (sprite top-left) to the body origin.
    half_extents: (f32, f32),
    /// Fixed bodies never move by themselves; nothing to write back.
    is_fixed: bool,
    /// Dynamic bodies that may rotate write their angle back to `rotation`.
    writes_rotation: bool,
    /// Pulled by `creates_gravity` fields.
    affected_by_gravity: bool,
    /// Optional `position` Vector2 attribute, kept in sync with `x`/`y`.
    position_attr: Option<Uuid>,
}

pub struct PhysicsEngine {
    world: PhysicsWorld,
    bodies: HashMap<Uuid, BodyInfo>,
    time_step: f32,
    /// Real duration of the current rendered frame (see `begin_frame`).
    frame_dt: f32,
    /// Linear and angular impulses from `apply_force` / `apply_torque`,
    /// applied at the next simulation step.
    pending_impulses: HashMap<Uuid, (Vector, f32)>,
    /// Invalid attribute values already reported, so entities added again
    /// and again (spawned copies) don't flood the console.
    reported_problems: HashSet<String>,
}

impl Default for PhysicsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsEngine {
    pub fn new() -> Self {
        Self {
            world: new_world(
                Vector::new(DEFAULT_GRAVITY.0, DEFAULT_GRAVITY.1),
                DEFAULT_TIME_STEP,
            ),
            bodies: HashMap::new(),
            time_step: DEFAULT_TIME_STEP,
            frame_dt: DEFAULT_TIME_STEP,
            pending_impulses: HashMap::new(),
            reported_problems: HashSet::new(),
        }
    }

    /// Set the global gravity vector (screen space: +y is down).
    pub fn set_gravity(&mut self, x: f32, y: f32) {
        self.world.gravity = Vector::new(x, y);
    }

    /// Current global gravity vector.
    pub fn get_gravity(&self) -> (f32, f32) {
        (self.world.gravity.x, self.world.gravity.y)
    }

    /// Set the fixed simulation step, in seconds. Non-positive or
    /// non-finite values are ignored.
    pub fn set_time_step(&mut self, time_step: f32) {
        if time_step > 0.0 && time_step.is_finite() {
            self.time_step = time_step;
            self.world.integration_parameters.dt = time_step;
        }
    }

    pub fn get_time_step(&self) -> f32 {
        self.time_step
    }

    /// Mirror an entity into the physics world, replacing its previous body
    /// if it already has one.
    ///
    /// Entities take part only if they have at least one of the
    /// `has_gravity`, `has_collision` or `creates_gravity` attributes.
    pub fn add_entity(&mut self, entity: &Entity) {
        let participates = ["has_gravity", "has_collision", "creates_gravity"]
            .iter()
            .any(|name| entity.get_attribute_by_name(name).is_ok());
        if !participates {
            return;
        }
        if self.bodies.contains_key(&entity.id) {
            self.remove_entity(entity.id);
        }

        let is_movable = attr_bool(entity, "is_movable", false);
        let has_gravity = attr_bool(entity, "has_gravity", false);
        let has_collision = attr_bool(entity, "has_collision", true);
        let can_rotate = attr_bool(entity, "can_rotate", false);
        let is_kinematic = attr_bool(entity, "is_kinematic", false);
        let gravity_scale = attr_float(entity, "gravity_scale", 1.0);
        let rotation = attr_float(entity, "rotation", 0.0).to_radians();

        let size = body_size(entity);
        let half_extents = size.map_or((0.0, 0.0), |(w, h)| (w / 2.0, h / 2.0));
        let origin = Vector::new(
            entity.get_x() + half_extents.0,
            entity.get_y() + half_extents.1,
        );

        // - kinematic: moved only via set_velocity; ignores gravity, forces
        //   and pushes from dynamic bodies (script-driven obstacles)
        // - dynamic (is_movable): full simulation
        // - fixed: static geometry
        let builder = if is_kinematic {
            // A sleeping kinematic body would freeze mid-motion: a constant
            // velocity doesn't wake it up
            RigidBodyBuilder::kinematic_velocity_based().can_sleep(false)
        } else if is_movable {
            let mut builder = RigidBodyBuilder::dynamic().gravity_scale(if has_gravity {
                gravity_scale
            } else {
                0.0
            });
            if !can_rotate {
                builder = builder.lock_rotations();
            }
            if !has_collision {
                // Mass normally comes from the collider; without one, gravity
                // and forces would have nothing to act on
                builder = builder.additional_mass(1.0);
            }
            builder
        } else {
            RigidBodyBuilder::fixed()
        };
        let body = self.world.insert_body(
            builder
                .translation(origin)
                .rotation(rotation)
                .user_data(entity.id.as_u128())
                .build(),
        );

        let is_sensor = attr_bool(entity, "is_sensor", false);
        let (groups, problems) = collision_groups(entity);
        for problem in problems {
            if self.reported_problems.insert(problem.clone()) {
                LOGGER.warning(problem);
            }
        }
        let collider = has_collision.then(|| {
            let mut collider = collider_builder(entity, size)
                .density(attr_float(entity, "density", 1.0))
                .friction(attr_float(entity, "friction", 0.5))
                .restitution(attr_float(entity, "restitution", 0.0))
                .sensor(is_sensor)
                .collision_groups(groups)
                .user_data(entity.id.as_u128());
            if is_sensor {
                // Trigger zones notice every kind of body, including
                // kinematic ones entering fixed zones
                collider = collider.active_collision_types(ActiveCollisionTypes::all());
            }
            self.world.insert_collider(collider.build(), Some(body))
        });

        self.bodies.insert(
            entity.id,
            BodyInfo {
                body,
                collider,
                half_extents,
                is_fixed: !is_kinematic && !is_movable,
                writes_rotation: is_movable && !is_kinematic && can_rotate,
                affected_by_gravity: has_gravity,
                position_attr: entity.get_attribute_by_name("position").ok().map(|a| a.id),
            },
        );
    }

    /// Remove an entity's body and collider. Does nothing if it has none.
    pub fn remove_entity(&mut self, entity_id: Uuid) {
        self.pending_impulses.remove(&entity_id);
        if let Some(info) = self.bodies.remove(&entity_id) {
            // Also removes the attached collider
            self.world.remove_body(info.body);
        }
    }

    /// Mirror every qualifying entity of the scene into the physics world.
    pub fn load_scene(&mut self, scene: &Scene) {
        for entity in scene.entities.values() {
            self.add_entity(entity);
        }
    }

    /// Start a rendered frame lasting `frame_dt` seconds. Forces applied
    /// during the frame are integrated over this duration, which keeps
    /// their effect independent of the display refresh rate.
    pub fn begin_frame(&mut self, frame_dt: f32) {
        if frame_dt > 0.0 && frame_dt.is_finite() {
            self.frame_dt = frame_dt;
        }
    }

    /// Advance the simulation by one fixed step and write the resulting
    /// positions (and rotations of rotating bodies) back into the scene.
    pub fn step(&mut self, scene: &mut Scene) {
        // Bodies whose entity is gone (e.g. removed by a script) must not
        // keep colliding as invisible obstacles
        let orphaned: Vec<Uuid> = self
            .bodies
            .keys()
            .filter(|id| !scene.entities.contains_key(*id))
            .copied()
            .collect();
        for id in orphaned {
            self.remove_entity(id);
        }

        for (id, (linear, angular)) in std::mem::take(&mut self.pending_impulses) {
            if let Some(rb) = self.body_mut(id) {
                rb.apply_impulse(linear, true);
                rb.apply_torque_impulse(angular, true);
            }
        }

        self.apply_gravity_fields(scene);
        self.world.step();

        // Field forces are persistent in rapier; they're recomputed each step
        for (_, rb) in self.world.bodies.iter_mut() {
            rb.reset_forces(false);
            rb.reset_torques(false);
        }

        for (id, info) in &self.bodies {
            if info.is_fixed {
                continue;
            }
            let (Some(rb), Some(entity)) =
                (self.world.bodies.get(info.body), scene.entities.get_mut(id))
            else {
                continue;
            };
            let origin = rb.translation();
            let x = origin.x - info.half_extents.0;
            let y = origin.y - info.half_extents.1;
            if !x.is_finite() || !y.is_finite() {
                continue;
            }
            let _ = entity.set_x(x);
            let _ = entity.set_y(y);
            if let Some(position_attr) = info.position_attr {
                let _ = entity.modify_attribute(
                    position_attr,
                    None,
                    None,
                    Some(AttributeValue::Vector2(x, y)),
                );
            }
            if info.writes_rotation {
                let degrees = rb.rotation().angle().to_degrees();
                if degrees.is_finite() {
                    set_float_attribute(entity, "rotation", degrees);
                }
            }
        }
    }

    /// Remove every body and collider and restore the default gravity.
    /// The time step is kept.
    pub fn cleanup(&mut self) {
        self.bodies.clear();
        self.pending_impulses.clear();
        self.reported_problems.clear();
        self.world = new_world(
            Vector::new(DEFAULT_GRAVITY.0, DEFAULT_GRAVITY.1),
            self.time_step,
        );
    }

    pub fn get_velocity(&self, entity_id: Uuid) -> Option<(f32, f32)> {
        self.body(entity_id).map(|rb| {
            let v = rb.linvel();
            (v.x, v.y)
        })
    }

    pub fn set_velocity(&mut self, entity_id: Uuid, velocity: (f32, f32)) {
        if let Some(rb) = self.body_mut(entity_id) {
            rb.set_linvel(Vector::new(velocity.0, velocity.1), true);
        }
    }

    /// Push the body with a force for the duration of the current frame
    /// (see [`begin_frame`](Self::begin_frame)). Call it every frame for a
    /// continuous force.
    pub fn apply_force(&mut self, entity_id: Uuid, force: (f32, f32)) {
        if self.bodies.contains_key(&entity_id) {
            let impulse = Vector::new(force.0, force.1) * self.frame_dt;
            self.pending_impulses.entry(entity_id).or_default().0 += impulse;
        }
    }

    /// Apply an instantaneous change in momentum.
    pub fn apply_impulse(&mut self, entity_id: Uuid, impulse: (f32, f32)) {
        if let Some(rb) = self.body_mut(entity_id) {
            rb.apply_impulse(Vector::new(impulse.0, impulse.1), true);
        }
    }

    /// Teleport a body so the entity's top-left corner is at `(x, y)`.
    /// Velocity is kept.
    pub fn set_position(&mut self, entity_id: Uuid, x: f32, y: f32) {
        let Some(info) = self.bodies.get(&entity_id) else {
            return;
        };
        let origin = Vector::new(x + info.half_extents.0, y + info.half_extents.1);
        if let Some(rb) = self.world.bodies.get_mut(info.body) {
            rb.set_translation(origin, true);
        }
    }

    pub fn get_angular_velocity(&self, entity_id: Uuid) -> Option<f32> {
        self.body(entity_id).map(|rb| rb.angvel())
    }

    pub fn set_angular_velocity(&mut self, entity_id: Uuid, angular_velocity: f32) {
        if let Some(rb) = self.body_mut(entity_id) {
            rb.set_angvel(angular_velocity, true);
        }
    }

    /// Spin the body with a torque for the duration of the current frame,
    /// like [`apply_force`](Self::apply_force).
    pub fn apply_torque(&mut self, entity_id: Uuid, torque: f32) {
        if self.bodies.contains_key(&entity_id) {
            self.pending_impulses.entry(entity_id).or_default().1 += torque * self.frame_dt;
        }
    }

    /// Entities whose colliders currently touch this entity's collider, or
    /// overlap it when either is a sensor.
    pub fn get_colliding_entities(&self, entity_id: Uuid) -> Vec<Uuid> {
        let Some(collider) = self.bodies.get(&entity_id).and_then(|info| info.collider) else {
            return Vec::new();
        };
        let contacts = self
            .world
            .narrow_phase
            .contact_pairs_with(collider)
            // Pairs also exist for shapes that are merely close; only
            // report actual touching
            .filter(|pair| pair.has_any_active_contact())
            .filter_map(|pair| {
                let other = if pair.collider1 == collider {
                    pair.collider2
                } else {
                    pair.collider1
                };
                self.world.colliders.get(other)
            });
        // Sensors don't touch: they overlap
        let overlaps = self
            .world
            .intersection_pairs_with(collider)
            .filter(|(.., intersecting)| *intersecting)
            .map(|(handle1, collider1, _, collider2, _)| {
                if handle1 == collider {
                    collider2
                } else {
                    collider1
                }
            });
        contacts
            .chain(overlaps)
            .map(|other| Uuid::from_u128(other.user_data))
            .collect()
    }

    /// The first collider the segment from `from` to `to` hits, ignoring
    /// sensors and, if given, the body of `exclude` (e.g. the entity casting
    /// the ray). Colliders added since the last [`step`](Self::step) aren't
    /// found yet.
    pub fn raycast(
        &self,
        from: (f32, f32),
        to: (f32, f32),
        exclude: Option<Uuid>,
    ) -> Option<RayHit> {
        let origin = Vector::new(from.0, from.1);
        let delta = Vector::new(to.0 - from.0, to.1 - from.1);
        let length = delta.length();
        if !(length.is_finite() && length > 0.0) {
            return None;
        }
        let ray = Ray::new(origin, delta / length);
        let mut filter = QueryFilter::default().exclude_sensors();
        if let Some(info) = exclude.and_then(|id| self.bodies.get(&id)) {
            filter = filter.exclude_rigid_body(info.body);
        }
        let (handle, distance) = self.world.cast_ray(&ray, length, true, filter)?;
        let collider = self.world.colliders.get(handle)?;
        let point = ray.point_at(distance);
        Some(RayHit {
            entity_id: Uuid::from_u128(collider.user_data),
            point: (point.x, point.y),
            distance,
        })
    }

    /// World-space outlines of all colliders, for debug drawing.
    pub fn collider_outlines(&self) -> Vec<ColliderOutline> {
        self.bodies
            .values()
            .filter_map(|info| self.world.colliders.get(info.collider?))
            .filter_map(|collider| {
                let center = collider.translation();
                let rotation = collider.rotation().angle();
                let shape = collider.shape();
                let (size, shape) = if let Some(ball) = shape.as_ball() {
                    let diameter = ball.radius * 2.0;
                    ((diameter, diameter), ColliderShape::Circle)
                } else {
                    let cuboid = shape.as_cuboid()?;
                    let extents = cuboid.half_extents * 2.0;
                    ((extents.x, extents.y), ColliderShape::Rectangle)
                };
                Some(ColliderOutline {
                    center: (center.x, center.y),
                    size,
                    rotation,
                    shape,
                    is_sensor: collider.is_sensor(),
                })
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.bodies.is_empty()
    }

    /// Total number of rigid bodies in the physics world (including any no
    /// longer mapped to an entity - useful to detect leaks).
    pub fn rigid_body_count(&self) -> usize {
        self.world.bodies.len()
    }

    pub fn has_rigid_body(&self, entity_id: Uuid) -> bool {
        self.bodies.contains_key(&entity_id)
    }

    pub fn has_collider(&self, entity_id: Uuid) -> bool {
        self.bodies
            .get(&entity_id)
            .is_some_and(|info| info.collider.is_some())
    }

    fn body(&self, entity_id: Uuid) -> Option<&RigidBody> {
        let info = self.bodies.get(&entity_id)?;
        self.world.bodies.get(info.body)
    }

    fn body_mut(&mut self, entity_id: Uuid) -> Option<&mut RigidBody> {
        let info = self.bodies.get(&entity_id)?;
        self.world.bodies.get_mut(info.body)
    }

    /// Attract bodies with `has_gravity` towards every `creates_gravity`
    /// entity: acceleration `gravity_strength / distance²`, towards the
    /// source's body center (or its `x`/`y` if it has no body).
    fn apply_gravity_fields(&mut self, scene: &Scene) {
        let sources: Vec<(Uuid, Vector, f32)> = scene
            .entities
            .values()
            .filter(|entity| attr_bool(entity, "creates_gravity", false))
            .map(|entity| {
                let position = self
                    .body(entity.id)
                    .map(|rb| rb.translation())
                    .unwrap_or_else(|| Vector::new(entity.get_x(), entity.get_y()));
                let strength =
                    attr_float(entity, "gravity_strength", DEFAULT_GRAVITY_FIELD_STRENGTH);
                (entity.id, position, strength)
            })
            .collect();
        if sources.is_empty() {
            return;
        }

        for (id, info) in &self.bodies {
            if !info.affected_by_gravity {
                continue;
            }
            let Some(rb) = self.world.bodies.get_mut(info.body) else {
                continue;
            };
            if !rb.is_dynamic() {
                continue;
            }
            let position = rb.translation();
            let mut acceleration = Vector::ZERO;
            for (source_id, source_position, strength) in &sources {
                if source_id == id {
                    continue;
                }
                let offset = *source_position - position;
                let distance = offset.length().max(MIN_GRAVITY_FIELD_DISTANCE);
                acceleration += offset.normalize_or_zero() * (strength / (distance * distance));
            }
            let force = acceleration * rb.mass();
            rb.add_force(force, true);
        }
    }
}

fn new_world(gravity: Vector, time_step: f32) -> PhysicsWorld {
    PhysicsWorld {
        gravity,
        integration_parameters: IntegrationParameters {
            dt: time_step,
            // The engine works in pixels; typical sprites are ~100px. This
            // scales rapier's internal tolerances (sleep thresholds, allowed
            // penetration, prediction distance) accordingly - without it,
            // large slow-moving bodies fall asleep mid-motion.
            length_unit: 100.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Size of the entity's body: explicit `collider_width`/`collider_height`
/// attributes, otherwise the first image's pixel size (one frame's, for a
/// sprite sheet) times the `scale` attribute, matching what the renderer
/// draws.
fn body_size(entity: &Entity) -> Option<(f32, f32)> {
    let explicit_width = entity.get_attribute_by_name("collider_width").ok();
    let explicit_height = entity.get_attribute_by_name("collider_height").ok();
    if let (Some(width), Some(height)) = (explicit_width, explicit_height)
        && let (AttributeValue::Float(w), AttributeValue::Float(h)) = (&width.value, &height.value)
    {
        return Some((w.abs(), h.abs()));
    }

    let image_path = entity.get_image(0).ok()?;
    // Reads only the image header
    let image_size = image::image_dimensions(image_path).ok()?;
    let (width, height) = match crate::render::frame_size(entity) {
        // A sprite sheet: one frame, as the renderer shows it
        Some(frame_size) => crate::render::sheet_frame(image_size, frame_size, 0).1,
        None => (image_size.0 as f32, image_size.1 as f32),
    };
    let (scale_x, scale_y) = entity.get_vector2("scale").unwrap_or((1.0, 1.0));
    Some(((width * scale_x).abs(), (height * scale_y).abs()))
}

/// Collider centered on the body origin. The shape comes from the
/// `collider_shape` attribute ("circle" or "rectangle", default rectangle);
/// circles fit inside the body size. Entities with no known size get a
/// small ball so they still collide.
fn collider_builder(entity: &Entity, size: Option<(f32, f32)>) -> ColliderBuilder {
    let Some((width, height)) = size.filter(|(w, h)| *w > 0.0 && *h > 0.0) else {
        return ColliderBuilder::ball(0.5);
    };
    let wants_circle = matches!(
        entity.get_attribute_by_name("collider_shape").map(|a| &a.value),
        Ok(AttributeValue::String(shape)) if shape.eq_ignore_ascii_case("circle")
    );
    if wants_circle {
        ColliderBuilder::ball(width.min(height) / 2.0)
    } else {
        ColliderBuilder::cuboid(width / 2.0, height / 2.0)
    }
}

/// Collision filtering from two attributes: `collision_layer` (Integer from
/// 1 to 32, default 1) and `collides_with` (String of comma-separated layer
/// numbers; empty or missing means every layer). Two entities interact only
/// if each one's layer is in the other's `collides_with`. Invalid values are
/// replaced by the defaults and described in the returned problems.
fn collision_groups(entity: &Entity) -> (InteractionGroups, Vec<String>) {
    let mut problems = Vec::new();
    let layer_bit = |layer: i64| Group::from_bits_truncate(1 << (layer - 1));
    let layer = match entity.get_float("collision_layer") {
        None => 1,
        Some(layer) if layer.fract() == 0.0 && (1.0..=32.0).contains(&layer) => layer as i64,
        Some(layer) => {
            problems.push(format!(
                "Entity '{}': collision_layer must be a layer from 1 to 32, not {}",
                entity.name, layer
            ));
            1
        }
    };
    let filter = match entity.get_string("collides_with").map(str::trim) {
        None | Some("") => Group::ALL,
        Some(list) => {
            let layers: Result<Vec<i64>, _> = list
                .split(',')
                .map(|part| part.trim().parse::<i64>())
                .collect();
            match layers {
                Ok(layers) if layers.iter().all(|layer| (1..=32).contains(layer)) => layers
                    .into_iter()
                    .fold(Group::NONE, |groups, layer| groups | layer_bit(layer)),
                _ => {
                    problems.push(format!(
                        "Entity '{}': collides_with must list layers from 1 to 32 \
                         (like \"1,3\"), not \"{}\"; it collides with every layer",
                        entity.name, list
                    ));
                    Group::ALL
                }
            }
        }
    };
    (
        InteractionGroups::new(layer_bit(layer), filter, InteractionTestMode::And),
        problems,
    )
}

fn attr_bool(entity: &Entity, name: &str, default: bool) -> bool {
    match entity.get_attribute_by_name(name).map(|a| &a.value) {
        Ok(AttributeValue::Boolean(value)) => *value,
        _ => default,
    }
}

fn attr_float(entity: &Entity, name: &str, default: f32) -> f32 {
    match entity.get_attribute_by_name(name).map(|a| &a.value) {
        Ok(AttributeValue::Float(value)) if value.is_finite() => *value,
        Ok(AttributeValue::Integer(value)) => *value as f32,
        _ => default,
    }
}

/// Set a Float attribute, creating it if the entity doesn't have it yet.
fn set_float_attribute(entity: &mut Entity, name: &str, value: f32) {
    match entity.get_attribute_by_name(name).map(|a| a.id) {
        Ok(id) => {
            let _ = entity.modify_attribute(id, None, None, Some(AttributeValue::Float(value)));
        }
        Err(_) => {
            let _ =
                entity.create_attribute(name, AttributeType::Float, AttributeValue::Float(value));
        }
    }
}
