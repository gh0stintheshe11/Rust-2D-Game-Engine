//! 2D physics: a rapier2d world mirroring the scene's physics entities.
//!
//! Coordinates are screen-space pixels with +Y pointing down. An entity's
//! `x`/`y` attributes are the top-left corner of its sprite; its rigid body
//! is centered on the sprite (or on the explicit collider size), so bodies
//! rotate around the sprite's center like the renderer does.

use crate::logger::LOGGER;
use crate::scene::{AttributeType, AttributeValue, Entity, Scene, TileCollision, Tilemap};
use rapier2d::math::IVector;
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

/// A one-way platform holds bodies whose contact with it points at most
/// this far (radians) from straight up: landing on its corner counts too.
const ONE_WAY_ALLOWED_ANGLE: f32 = std::f32::consts::FRAC_PI_4;

/// The contact state rapier's `update_as_oneway_platform` keeps in a
/// manifold's `user_data` for "refused until the bodies are apart": a body
/// that dropped through a platform isn't popped back on top when it stops
/// dropping while still inside.
const ONE_WAY_REFUSED: u32 = 2;

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
    /// A one-way platform (a tilemap's one-way tile), solid only from above.
    pub one_way: bool,
}

/// Bookkeeping for one entity mirrored into the physics world.
struct BodyInfo {
    body: RigidBodyHandle,
    collider: Option<ColliderHandle>,
    /// A tilemap's one-way tiles, apart from its solid ones (`collider`)
    one_way_collider: Option<ColliderHandle>,
    /// A tilemap's voxels in `collider` and in `one_way_collider`: a
    /// collider left without any goes
    voxel_counts: [usize; 2],
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

impl BodyInfo {
    /// The body's colliders: at most one, except for a tilemap with solid
    /// and one-way tiles.
    fn colliders(&self) -> impl Iterator<Item = ColliderHandle> + use<> {
        self.collider.into_iter().chain(self.one_way_collider)
    }
}

/// Whether a collider is a one-way platform (a tilemap's one-way tiles).
fn is_one_way(colliders: &ColliderSet, handle: ColliderHandle) -> bool {
    colliders.get(handle).is_some_and(|collider| {
        collider
            .active_hooks()
            .contains(ActiveHooks::MODIFY_SOLVER_CONTACTS)
    })
}

/// Physics hooks making colliders with
/// [`ActiveHooks::MODIFY_SOLVER_CONTACTS`] (a tilemap's one-way tiles) solid
/// only for bodies coming from above: a body jumps up through them, lands on
/// them, and walks through them sideways. Bodies in `dropping` fall through
/// them (see [`PhysicsEngine::drop_through_platforms`]).
struct OneWayPlatforms<'a> {
    dropping: &'a HashSet<RigidBodyHandle>,
}

impl PhysicsHooks for OneWayPlatforms<'_> {
    fn modify_solver_contacts(&self, context: &mut ContactModificationContext) {
        let colliders = context.colliders;
        // Contact normals point out of collider1: the allowed one points up
        // (-y) out of a platform, or down into it
        let (allowed, other_body) = if is_one_way(colliders, context.collider1) {
            (Vector::new(0.0, -1.0), context.rigid_body2)
        } else if is_one_way(colliders, context.collider2) {
            (Vector::new(0.0, 1.0), context.rigid_body1)
        } else {
            return;
        };
        if other_body.is_some_and(|body| self.dropping.contains(&body)) {
            if let Some(rigid) = context.rigid_mut() {
                rigid.solver_contacts.clear();
                *rigid.user_data = ONE_WAY_REFUSED;
            }
            return;
        }
        let Some(collider1) = colliders.get(context.collider1) else {
            return;
        };
        // ...in collider1's frame, which a rotating body turns
        let allowed = collider1.rotation().inverse() * allowed;
        context.update_as_oneway_platform(allowed, ONE_WAY_ALLOWED_ANGLE);
    }
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
    /// Bodies falling through one-way platforms, until they're clear of them
    dropping: HashSet<RigidBodyHandle>,
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
            dropping: HashSet::new(),
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
            .any(|name| entity.attribute(name).is_some());
        if !participates {
            return;
        }
        if self.bodies.contains_key(&entity.id) {
            self.remove_entity(entity.id);
        }
        if let Some(tilemap) = &entity.tilemap {
            self.add_tilemap(entity, tilemap);
            return;
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

        let collider = has_collision
            .then(|| self.insert_collider(entity, collider_builder(entity, size), body));

        self.bodies.insert(
            entity.id,
            BodyInfo {
                body,
                collider,
                one_way_collider: None,
                voxel_counts: [0, 0],
                half_extents,
                is_fixed: !is_kinematic && !is_movable,
                writes_rotation: is_movable && !is_kinematic && can_rotate,
                affected_by_gravity: has_gravity,
                position_attr: entity.attribute("position").map(|a| a.id),
            },
        );
    }

    /// Remove an entity's body and collider. Does nothing if it has none.
    pub fn remove_entity(&mut self, entity_id: Uuid) {
        self.pending_impulses.remove(&entity_id);
        if let Some(info) = self.bodies.remove(&entity_id) {
            self.dropping.remove(&info.body);
            // Also removes the attached collider
            self.world.remove_body(info.body);
        }
    }

    /// Let an entity's body fall through the one-way platforms it stands
    /// on, or touches: they stop holding it until it's clear of all of
    /// them, and hold it as usual again after. A body touching none is left
    /// as it is. Returns whether the entity has a body.
    pub fn drop_through_platforms(&mut self, entity_id: Uuid) -> bool {
        let Some(info) = self.bodies.get(&entity_id) else {
            return false;
        };
        let body = info.body;
        self.dropping.insert(body);
        // Resting bodies sleep
        self.world.wake_up(body, true);
        true
    }

    /// Stop the drops of the bodies clear of every one-way platform.
    fn end_finished_drops(&mut self) {
        let finished: Vec<RigidBodyHandle> = self
            .dropping
            .iter()
            .filter(|body| !self.touches_one_way_platform(**body))
            .copied()
            .collect();
        for body in finished {
            self.dropping.remove(&body);
        }
    }

    /// Whether a body has contact points with a one-way platform, even ones
    /// the platform refuses.
    fn touches_one_way_platform(&self, body: RigidBodyHandle) -> bool {
        let Some(rigid_body) = self.world.bodies.get(body) else {
            return false;
        };
        rigid_body.colliders().iter().any(|&collider| {
            self.world
                .narrow_phase
                .contact_pairs_with(collider)
                .any(|pair| {
                    let other = if pair.collider1 == collider {
                        pair.collider2
                    } else {
                        pair.collider1
                    };
                    is_one_way(&self.world.colliders, other)
                        && pair
                            .manifolds()
                            .iter()
                            .any(|manifold| !manifold.points.is_empty())
                })
        })
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
        let hooks = OneWayPlatforms {
            dropping: &self.dropping,
        };
        self.world.step_with_events(&hooks, &());
        self.end_finished_drops();

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
        self.dropping.clear();
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

    /// Where the entity's body is: the middle of its collider (a tilemap's
    /// top-left corner); `None` without a body.
    pub fn body_center(&self, entity_id: Uuid) -> Option<(f32, f32)> {
        let info = self.bodies.get(&entity_id)?;
        let translation = self.world.bodies.get(info.body)?.translation();
        Some((translation.x, translation.y))
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

    /// A tilemap's body: fixed with its origin at the entity's `x`/`y` (the
    /// top-left corner of cell (0, 0)), and with `has_collision` colliding
    /// where the map has tiles, as each tile's [`TileCollision`] says. The
    /// solid cells form one voxel collider, which keeps bodies sliding along
    /// a row of tiles from catching on the edges between them; the one-way
    /// cells form another, which [`OneWayPlatforms`] makes solid only from
    /// above.
    fn add_tilemap(&mut self, entity: &Entity, tilemap: &Tilemap) {
        let origin = Vector::new(entity.get_x(), entity.get_y());
        let body = self.world.insert_body(
            RigidBodyBuilder::fixed()
                .translation(origin)
                .user_data(entity.id.as_u128())
                .build(),
        );
        // The cells of solid tiles, and of one-way tiles. A trigger zone has
        // no side to land on: its one-way tiles are part of the zone.
        let is_sensor = attr_bool(entity, "is_sensor", false);
        let (mut solid, mut one_way) = (Vec::new(), Vec::new());
        if attr_bool(entity, "has_collision", true) {
            for ((column, row), tile) in tilemap.tiles() {
                let cell = IVector::new(column, row);
                match tilemap.tile_collision(tile) {
                    TileCollision::Solid => solid.push(cell),
                    TileCollision::OneWay if is_sensor => solid.push(cell),
                    TileCollision::OneWay => one_way.push(cell),
                    TileCollision::Passable => {}
                }
            }
        }
        let tile = Vector::new(tilemap.tile_size.0 as f32, tilemap.tile_size.1 as f32);
        let collider = (!solid.is_empty())
            .then(|| self.insert_collider(entity, ColliderBuilder::voxels(tile, &solid), body));
        let one_way_collider = (!one_way.is_empty()).then(|| {
            let builder = ColliderBuilder::voxels(tile, &one_way)
                .active_hooks(ActiveHooks::MODIFY_SOLVER_CONTACTS);
            self.insert_collider(entity, builder, body)
        });
        // The body is fixed, so its mass doesn't matter, and working it out
        // goes through every tile: rapier would after every change of tiles
        for handle in collider.into_iter().chain(one_way_collider) {
            if let Some(collider) = self.world.colliders.get_mut(handle) {
                collider.set_density(0.0);
            }
        }
        self.bodies.insert(
            entity.id,
            BodyInfo {
                body,
                collider,
                one_way_collider,
                voxel_counts: [solid.len(), one_way.len()],
                half_extents: (0.0, 0.0),
                is_fixed: true,
                writes_rotation: false,
                affected_by_gravity: false,
                position_attr: None,
            },
        );
    }

    /// Attach a collider shaped by `builder` to `body`, with the entity's
    /// material (`density`, `friction`, `restitution`), `is_sensor` and
    /// collision layers.
    fn insert_collider(
        &mut self,
        entity: &Entity,
        builder: ColliderBuilder,
        body: RigidBodyHandle,
    ) -> ColliderHandle {
        let is_sensor = attr_bool(entity, "is_sensor", false);
        let (groups, problems) = collision_groups(entity);
        for problem in problems {
            if self.reported_problems.insert(problem.clone()) {
                LOGGER.warning(problem);
            }
        }
        let mut builder = builder
            .density(attr_float(entity, "density", 1.0))
            .friction(attr_float(entity, "friction", 0.5))
            .restitution(attr_float(entity, "restitution", 0.0))
            .sensor(is_sensor)
            .collision_groups(groups)
            .user_data(entity.id.as_u128());
        if is_sensor {
            // Trigger zones notice every kind of body, including kinematic
            // ones entering fixed zones
            builder = builder.active_collision_types(ActiveCollisionTypes::all());
        }
        self.world.insert_collider(builder.build(), Some(body))
    }

    /// Bring a tilemap's colliders up to date after the tiles of `cells`
    /// changed (a script broke a block, filled an area): only those cells'
    /// voxels change, which costs the same on a map of any size. Bodies
    /// touching the map around them wake up, so none stays floating where a
    /// tile was removed. When a kind of tile the map had none of appears
    /// (its first one-way tile, say), the colliders are rebuilt instead (see
    /// [`update_tilemap`](Self::update_tilemap)). Does nothing for an entity
    /// without a body.
    pub fn update_tilemap_cells(&mut self, entity: &Entity, cells: &[(i32, i32)]) {
        let (Some(tilemap), Some(info)) = (&entity.tilemap, self.bodies.get(&entity.id)) else {
            return;
        };
        if cells.is_empty() {
            return;
        }
        let has_collision = attr_bool(entity, "has_collision", true);
        let is_sensor = attr_bool(entity, "is_sensor", false);
        // Whether a cell has a voxel in the solid collider, and in the
        // one-way one (see add_tilemap)
        let wanted = |(column, row): (i32, i32)| -> [bool; 2] {
            let tile = tilemap.get(column, row).filter(|_| has_collision);
            match tile.map(|tile| tilemap.tile_collision(tile)) {
                Some(TileCollision::Solid) => [true, false],
                Some(TileCollision::OneWay) if is_sensor => [true, false],
                Some(TileCollision::OneWay) => [false, true],
                Some(TileCollision::Passable) | None => [false, false],
            }
        };
        let handles = [info.collider, info.one_way_collider];
        let missing = cells.iter().any(|&cell| {
            let wanted = wanted(cell);
            (wanted[0] && handles[0].is_none()) || (wanted[1] && handles[1].is_none())
        });
        if missing {
            self.update_tilemap(entity);
            return;
        }

        // Bodies touching the map near the cells, found before a collider
        // may go
        let tile = (tilemap.tile_size.0 as f32, tilemap.tile_size.1 as f32);
        let (mut first, mut last) = (cells[0], cells[0]);
        for &(column, row) in cells {
            first = (first.0.min(column), first.1.min(row));
            last = (last.0.max(column), last.1.max(row));
        }
        let origin = (entity.get_x(), entity.get_y());
        // One tile around the cells
        let area = Aabb::new(
            Vector::new(
                origin.0 + (first.0 as f32 - 1.0) * tile.0,
                origin.1 + (first.1 as f32 - 1.0) * tile.1,
            ),
            Vector::new(
                origin.0 + (last.0 as f32 + 2.0) * tile.0,
                origin.1 + (last.1 as f32 + 2.0) * tile.1,
            ),
        );
        let touching = self.bodies_touching(handles.into_iter().flatten(), &area);

        let mut counts = info.voxel_counts;
        let mut emptied = Vec::new();
        for (slot, handle) in handles.into_iter().enumerate() {
            let Some(handle) = handle else {
                continue;
            };
            let Some(voxels) = self
                .world
                .colliders
                .get_mut(handle)
                .and_then(|collider| collider.shape_mut().as_voxels_mut())
            else {
                continue;
            };
            for &cell in cells {
                let filled = wanted(cell)[slot];
                let before = voxels.set_voxel(IVector::new(cell.0, cell.1), filled);
                match (before.is_empty(), filled) {
                    (true, true) => counts[slot] += 1,
                    (false, false) => counts[slot] = counts[slot].saturating_sub(1),
                    _ => {}
                }
            }
            if counts[slot] == 0 {
                emptied.push(handle);
            }
        }
        for handle in &emptied {
            self.world.remove_collider(*handle);
        }
        if let Some(info) = self.bodies.get_mut(&entity.id) {
            info.voxel_counts = counts;
            let gone = |handle: Option<ColliderHandle>| handle.filter(|h| !emptied.contains(h));
            info.collider = gone(info.collider);
            info.one_way_collider = gone(info.one_way_collider);
        }
        for body in touching {
            self.world.wake_up(body, true);
        }
    }

    /// The bodies of colliders in contact with `colliders` whose bounding
    /// box meets `area`.
    fn bodies_touching(
        &self,
        colliders: impl Iterator<Item = ColliderHandle>,
        area: &Aabb,
    ) -> Vec<RigidBodyHandle> {
        let world = &self.world;
        colliders
            .flat_map(|collider| {
                world
                    .narrow_phase
                    .contact_pairs_with(collider)
                    .filter_map(move |pair| {
                        let other = if pair.collider1 == collider {
                            pair.collider2
                        } else {
                            pair.collider1
                        };
                        let other = world.colliders.get(other)?;
                        other
                            .compute_aabb()
                            .intersects(area)
                            .then_some(other.parent()?)
                    })
            })
            .collect()
    }

    /// Rebuild a tilemap's colliders, e.g. after its tileset's collision
    /// changed. Bodies that touched it wake up, so none stays floating where
    /// a tile was removed. Does nothing for an entity without a body (one in
    /// a scene that isn't playing, say). After tiles changed,
    /// [`update_tilemap_cells`](Self::update_tilemap_cells) costs less.
    pub fn update_tilemap(&mut self, entity: &Entity) {
        let Some(info) = self.bodies.get(&entity.id) else {
            return;
        };
        let world = &self.world;
        let touching: Vec<RigidBodyHandle> = info
            .colliders()
            .flat_map(|collider| {
                world
                    .narrow_phase
                    .contact_pairs_with(collider)
                    .filter_map(move |pair| {
                        let other = if pair.collider1 == collider {
                            pair.collider2
                        } else {
                            pair.collider1
                        };
                        world.colliders.get(other)?.parent()
                    })
            })
            .collect();
        self.add_entity(entity);
        for body in touching {
            self.world.wake_up(body, true);
        }
    }

    /// Entities whose colliders currently touch this entity's collider, or
    /// overlap it when either is a sensor.
    pub fn get_colliding_entities(&self, entity_id: Uuid) -> Vec<Uuid> {
        let Some(info) = self.bodies.get(&entity_id) else {
            return Vec::new();
        };
        info.colliders()
            .flat_map(|collider| self.colliding_with(collider))
            .collect()
    }

    /// Entities whose colliders touch or overlap this collider.
    fn colliding_with(&self, collider: ColliderHandle) -> Vec<Uuid> {
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
        let mut outlines = Vec::new();
        let colliders = self
            .bodies
            .values()
            .flat_map(BodyInfo::colliders)
            .filter_map(|handle| self.world.colliders.get(handle));
        for collider in colliders {
            let center = collider.translation();
            let rotation = collider.rotation().angle();
            let one_way = collider
                .active_hooks()
                .contains(ActiveHooks::MODIFY_SOLVER_CONTACTS);
            let outline = |center: (f32, f32), size: (f32, f32), shape| ColliderOutline {
                center,
                size,
                rotation,
                shape,
                is_sensor: collider.is_sensor(),
                one_way,
            };
            let shape = collider.shape();
            if let Some(ball) = shape.as_ball() {
                let diameter = ball.radius * 2.0;
                outlines.push(outline(
                    (center.x, center.y),
                    (diameter, diameter),
                    ColliderShape::Circle,
                ));
            } else if let Some(cuboid) = shape.as_cuboid() {
                let extents = cuboid.half_extents * 2.0;
                outlines.push(outline(
                    (center.x, center.y),
                    (extents.x, extents.y),
                    ColliderShape::Rectangle,
                ));
            } else if let Some(voxels) = shape.as_voxels() {
                // A tilemap: one square per solid tile
                let size = voxels.voxel_size();
                let (sin, cos) = rotation.sin_cos();
                for voxel in voxels.voxels() {
                    let local = voxel.center;
                    let world = (
                        center.x + local.x * cos - local.y * sin,
                        center.y + local.x * sin + local.y * cos,
                    );
                    outlines.push(outline(world, (size.x, size.y), ColliderShape::Rectangle));
                }
            }
        }
        outlines
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
            .is_some_and(|info| info.colliders().next().is_some())
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
    let explicit_width = entity.attribute("collider_width");
    let explicit_height = entity.attribute("collider_height");
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
        entity.attribute("collider_shape").map(|a| &a.value),
        Some(AttributeValue::String(shape)) if shape.eq_ignore_ascii_case("circle")
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
    match entity.attribute(name).map(|a| &a.value) {
        Some(AttributeValue::Boolean(value)) => *value,
        _ => default,
    }
}

fn attr_float(entity: &Entity, name: &str, default: f32) -> f32 {
    match entity.attribute(name).map(|a| &a.value) {
        Some(AttributeValue::Float(value)) if value.is_finite() => *value,
        Some(AttributeValue::Integer(value)) => *value as f32,
        _ => default,
    }
}

/// Set a Float attribute, creating it if the entity doesn't have it yet.
fn set_float_attribute(entity: &mut Entity, name: &str, value: f32) {
    match entity.attribute(name).map(|a| a.id) {
        Some(id) => {
            let _ = entity.modify_attribute(id, None, None, Some(AttributeValue::Float(value)));
        }
        None => {
            let _ =
                entity.create_attribute(name, AttributeType::Float, AttributeValue::Float(value));
        }
    }
}
