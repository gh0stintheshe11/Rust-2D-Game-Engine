use rapier2d::prelude::*;
use uuid::Uuid;
use std::collections::HashMap;
use crate::ecs::{Scene, Entity, AttributeValue};
use image::GenericImageView;

pub struct PhysicsEngine {
    // Global gravity force applied to all dynamic bodies
    gravity: Vector<Real>,

    // Controls physics simulation timing and accuracy
    integration_parameters: IntegrationParameters,

    // Main physics simulation pipeline that coordinates all systems
    physics_pipeline: PhysicsPipeline,

    // Manages groups of interacting bodies (optimization for large scenes)
    island_manager: IslandManager,

    // Broad phase: Quick, rough check of which objects MIGHT be colliding
    // Uses spatial partitioning to avoid checking every object against every other object
    broad_phase: BroadPhaseMultiSap,

    // Narrow phase: Detailed collision detection between objects that broad phase found
    // Calculates exact collision points and forces
    narrow_phase: NarrowPhase,

    // Stores and manages all rigid bodies (physical objects that can move)
    rigid_body_set: RigidBodySet,

    // Stores and manages all colliders (shapes that define how objects collide)
    collider_set: ColliderSet,

    // For future: Handles physical joints/constraints between bodies
    impulse_joint_set: ImpulseJointSet,

    // For future: Handles more complex joint systems
    multibody_joint_set: MultibodyJointSet,

    // For future: Handles continuous collision detection for fast-moving objects
    ccd_solver: CCDSolver,

    // For future: Handles spatial queries like raycasts and shape intersections
    query_pipeline: QueryPipeline,

    // Maps our entity IDs to Rapier's physics handles
    entity_to_body: HashMap<Uuid, RigidBodyHandle>,
    entity_to_collider: HashMap<Uuid, ColliderHandle>,

    time_step: f32,  // Physics update time step in seconds

    // Store position attribute IDs for quick updates
    entity_position_attrs: HashMap<Uuid, Uuid>,
}

impl PhysicsEngine {
    pub fn new() -> Self {
        Self {
            // Default gravity points downward (-Y direction)
            gravity: vector![0.0, 50.0],

            // Physics runs at 60Hz (60 updates per second)
            integration_parameters: IntegrationParameters {
                dt: 1.0 / 60.0,
                min_ccd_dt: 1.0 / 60.0 / 100.0,
                contact_damping_ratio: 0.0,
                contact_natural_frequency: 30.0,
                joint_natural_frequency: 30.0,
                ..Default::default()
            },

            // Initialize all physics systems
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhaseMultiSap::new(),
            narrow_phase: NarrowPhase::new(),
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
            entity_to_body: HashMap::new(),
            entity_to_collider: HashMap::new(),
            time_step: 1.0 / 60.0,  // Default 60Hz physics
            entity_position_attrs: HashMap::new(),
        }
    }

    // Time step control
    pub fn set_time_step(&mut self, time_step: f32) {
        self.time_step = time_step;
        self.integration_parameters.dt = time_step;
    }

    pub fn get_time_step(&self) -> f32 {
        self.time_step
    }

    // CCD control
    pub fn set_min_ccd_dt(&mut self, min_dt: f32) {
        self.integration_parameters.min_ccd_dt = min_dt;
    }

    // Contact parameters
    pub fn set_contact_parameters(&mut self, damping: f32, frequency: f32) {
        self.integration_parameters.contact_damping_ratio = damping;
        self.integration_parameters.contact_natural_frequency = frequency;
    }

    // Joint parameters
    pub fn set_joint_frequency(&mut self, frequency: f32) {
        self.integration_parameters.joint_natural_frequency = frequency;
    }

    fn create_collider(&self, entity: &Entity, density: f32, friction: f32, restitution: f32) -> Collider {
        // Get first image path from entity (assuming first image is the sprite)
        let collider_builder = if let Ok(image_path) = entity.get_image(0) {
            // Get image dimensions
            if let Ok(img) = image::open(image_path) {
                let (width, height) = img.dimensions();

                let offset = vector![width as f32 / 2.0, height as f32 / 2.0];

                // If width and height are similar, use circle
                if (width as f32 / height as f32).abs() > 0.9
                   && (width as f32 / height as f32).abs() < 1.1 {
                    ColliderBuilder::ball(width as f32 / 2.0).translation(offset)
                } else {
                    // Otherwise use box
                    ColliderBuilder::cuboid(width as f32 / 2.0, height as f32 / 2.0).translation(offset)
                }
            } else {
                ColliderBuilder::ball(0.5) // Default if can't load image
            }
        } else {
            ColliderBuilder::ball(0.5) // Default if no image
        };

        // Add physics properties
        collider_builder
            .density(density)
            .friction(friction)
            .restitution(restitution)
            .build()
    }

    pub fn add_entity(&mut self, entity: &Entity) {

        let required_attributes = ["has_gravity", "has_collision", "creates_gravity"];
        let should_skip = required_attributes.iter().all(|attr_name| entity.get_attribute_by_name(attr_name).is_err());

        // Skip entities without the required attributes
        if should_skip {
            return;
        }

        // Store position attribute ID for quick updates
        if let Ok(pos_attr) = entity.get_attribute_by_name("position") {
            self.entity_position_attrs.insert(entity.id, pos_attr.id);
        }

        // Get physics properties from entity attributes
        let position = if let Ok(pos_attr) = entity.get_attribute_by_name("position") {
            if let AttributeValue::Vector2(x, y) = pos_attr.value {
                vector![x, y]
            } else {
                vector![0.0, 0.0]
            }
        } else {
            vector![0.0, 0.0]
        };

        let is_movable = entity.get_attribute_by_name("is_movable")
            .and_then(|attr| if let AttributeValue::Boolean(v) = attr.value { Ok(v) } else { Err("Attribute value is not a boolean".to_string()) })
            .unwrap_or(false);

        let affected_by_gravity = entity.get_attribute_by_name("has_gravity")
            .and_then(|attr| if let AttributeValue::Boolean(v) = attr.value { Ok(v) } else { Err("Attribute value is not a boolean".to_string()) })
            .unwrap_or(false);

        let has_collision = entity.get_attribute_by_name("has_collision")
            .and_then(|attr| if let AttributeValue::Boolean(v) = attr.value { Ok(v) } else { Err("Attribute value is not a boolean".to_string()) })
            .unwrap_or(true);

        let friction = entity.get_attribute_by_name("friction")
            .and_then(|attr| if let AttributeValue::Float(v) = attr.value { Ok(v) } else { Err("Attribute value is not a float".to_string()) })
            .unwrap_or(0.5);

        let restitution = entity.get_attribute_by_name("restitution")
            .and_then(|attr| if let AttributeValue::Float(v) = attr.value { Ok(v) } else { Err("Attribute value is not a float".to_string()) })
            .unwrap_or(0.0);

        let density = entity.get_attribute_by_name("density")
            .and_then(|attr| if let AttributeValue::Float(v) = attr.value { Ok(v) } else { Err("Attribute value is not a float".to_string()) })
            .unwrap_or(1.0);

        let can_rotate = entity.get_attribute_by_name("can_rotate")
            .and_then(|attr| if let AttributeValue::Boolean(v) = attr.value { Ok(v) } else { Err("Attribute value is not a boolean".to_string()) })
            .unwrap_or(false);

        // Create rigid body
        let rigid_body = if is_movable {
            let mut rb = RigidBodyBuilder::dynamic()
                .translation(position)
                .gravity_scale(if affected_by_gravity { 1.0 } else { 0.0 });

            if !can_rotate {
                rb = rb.lock_rotations();
            }

            rb.build()
        } else {
            RigidBodyBuilder::fixed()
                .translation(position)
                .build()
        };

        let rb_handle = self.rigid_body_set.insert(rigid_body);

        // Create collider with automatic shape detection
        if has_collision {
            let collider = self.create_collider(entity, density, friction, restitution);
            let collider_handle = self.collider_set
                .insert_with_parent(collider, rb_handle, &mut self.rigid_body_set);
            self.entity_to_collider.insert(entity.id, collider_handle);
        }

        self.entity_to_body.insert(entity.id, rb_handle);
    }

    pub fn remove_entity(&mut self, entity_id: Uuid) {
        self.entity_position_attrs.remove(&entity_id);
        if let Some(rb_handle) = self.entity_to_body.remove(&entity_id) {
            self.rigid_body_set.remove(
                rb_handle,
                &mut self.island_manager,
                &mut self.collider_set,
                &mut self.impulse_joint_set,
                &mut self.multibody_joint_set,
                true
            );
        }

        if let Some(collider_handle) = self.entity_to_collider.remove(&entity_id) {
            self.collider_set.remove(
                collider_handle,
                &mut self.island_manager,
                &mut self.rigid_body_set,
                true
            );
        }
    }

    pub fn step(&mut self, scene: &mut Scene) -> Vec<(Uuid, Uuid, AttributeValue)> {
        // Process custom gravity fields
        for (_, entity1) in &scene.entities {
            if let Ok(creates_gravity) = entity1.get_attribute_by_name("creates_gravity") {
                if let AttributeValue::Boolean(true) = creates_gravity.value {
                    let pos1 = if let Ok(pos) = entity1.get_attribute_by_name("position") {
                        if let AttributeValue::Vector2(x, y) = pos.value {
                            vector![x, y]
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    };

                    // Apply gravity to other entities
                    for (_, entity2) in &scene.entities {
                        if entity1.id == entity2.id {
                            continue;
                        }

                        if let Ok(affected_by_gravity) = entity2.get_attribute_by_name("has_gravity") {
                            if let AttributeValue::Boolean(true) = affected_by_gravity.value {
                                if let Some(rb_handle) = self.entity_to_body.get(&entity2.id) {
                                    if let Some(rb) = self.rigid_body_set.get_mut(*rb_handle) {
                                        let pos2 = rb.translation();
                                        let direction = pos1 - pos2;
                                        let distance = direction.norm();
                                        if distance > 0.0 {
                                            let force = direction * (1.0 / (distance * distance));
                                            rb.add_force(force * 10.0, true);  // Scale force as needed
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Run physics simulation
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &(),
        );

        // Update positions using stored attribute IDs
        let mut updates = Vec::new();

        for (entity_id, rb_handle) in &self.entity_to_body {
            if let Some(rb) = self.rigid_body_set.get(*rb_handle) {
                if let Some(pos_attr_id) = self.entity_position_attrs.get(entity_id) {
                    let position = rb.translation();
                    // println!("position: {:?}", position);
                    updates.push((
                        *entity_id,
                        *pos_attr_id,
                        AttributeValue::Vector2(position.x, position.y)
                    ));

                    // Also update the entity's x and y, these are used to render in the view
                    if let Some(entity) = scene.entities.get(entity_id) {
                        if let Ok(x_attr) = entity.get_attribute_by_name("x") {
                            updates.push((
                                *entity_id,
                                x_attr.id,
                                AttributeValue::Float(position.x),
                            ));
                        }

                        if let Ok(y_attr) = entity.get_attribute_by_name("y") {
                            updates.push((
                                *entity_id,
                                y_attr.id,
                                AttributeValue::Float(position.y),
                            ));
                        }
                    }

                }
            }
        }

        updates
    }

    pub fn load_scene(&mut self, scene: &Scene) {
        for (_, entity) in &scene.entities {
            self.add_entity(entity);
        }
    }

    // We should also add cleanup for scene switching
    pub fn cleanup(&mut self) {
        // Clear entity mappings
        self.entity_to_body.clear();
        self.entity_to_collider.clear();

        // Remove all physics objects
        self.rigid_body_set = RigidBodySet::new();
        self.collider_set = ColliderSet::new();

        // Reset physics state with new instances
        self.island_manager = IslandManager::new();
        self.broad_phase = BroadPhaseMultiSap::new();
        self.narrow_phase = NarrowPhase::new();
        self.impulse_joint_set = ImpulseJointSet::new();
        self.multibody_joint_set = MultibodyJointSet::new();
        self.ccd_solver = CCDSolver::new();
        self.query_pipeline = QueryPipeline::new();
    }

    // Get velocity of an entity
    pub fn get_velocity(&self, entity_id: &Uuid) -> Option<Vector<Real>> {
        self.entity_to_body.get(entity_id)
            .and_then(|rb_handle| self.rigid_body_set.get(*rb_handle))
            .map(|rb| rb.linvel().clone())
    }

    // Set velocity of an entity
    pub fn set_velocity(&mut self, entity_id: &Uuid, velocity: Vector<Real>) {
        if let Some(rb_handle) = self.entity_to_body.get(entity_id) {
            if let Some(rb) = self.rigid_body_set.get_mut(*rb_handle) {
                rb.set_linvel(velocity, true);
            }
        }
    }

    // Apply force to an entity
    pub fn apply_force(&mut self, entity_id: &Uuid, force: Vector<Real>) {
        if let Some(rb_handle) = self.entity_to_body.get(entity_id) {
            if let Some(rb) = self.rigid_body_set.get_mut(*rb_handle) {
                rb.add_force(force, true);
            }
        }
    }

    // Apply impulse (immediate force) to an entity
    pub fn apply_impulse(&mut self, entity_id: &Uuid, impulse: Vector<Real>) {
        if let Some(rb_handle) = self.entity_to_body.get(entity_id) {
            if let Some(rb) = self.rigid_body_set.get_mut(*rb_handle) {
                rb.apply_impulse(impulse, true);
            }
        }
    }

    // Get all entities colliding with this one
    pub fn get_colliding_entities(&self, entity_id: &Uuid) -> Vec<Uuid> {
        let mut colliding = Vec::new();

        if let Some(collider_handle) = self.entity_to_collider.get(entity_id) {
            let contact_pairs = self.narrow_phase.contact_pairs_with(*collider_handle);
            for pair in contact_pairs {
                let other_handle = if pair.collider1 == *collider_handle {
                    pair.collider2
                } else {
                    pair.collider1
                };

                // Find entity ID for this collider
                for (entity_id, &handle) in &self.entity_to_collider {
                    if handle == other_handle {
                        colliding.push(*entity_id);
                        break;
                    }
                }
            }
        }

        colliding
    }

    // Get all colliders and gives a shape for rendering
    // Returns:
    // - (f32, f32): The world coordinate of the collider (x, y).
    // - (f32, f32): The size of the collider in world coordinate (width, height).
    // - String: The shape of the collider (e.g., "Circle", "Rectangle").
    pub fn get_collider_data(&self) -> Vec<((f32, f32), (f32, f32), String)> {
        let mut colliders = Vec::new();

        for (entity_id, collider_handle) in &self.entity_to_collider {
            if let Some(collider) = self.collider_set.get(*collider_handle) {
                if let Some(rb_handle) = self.entity_to_body.get(entity_id) {
                    if let Some(rb) = self.rigid_body_set.get(*rb_handle) {
                        let position = (collider.translation().x, collider.translation().y);

                        if let Some(ball) = collider.shape().as_ball() {
                            colliders.push((position, (ball.radius * 2.0, ball.radius * 2.0), "Circle".to_string()));
                        } else if let Some(cuboid) = collider.shape().as_cuboid() {
                            colliders.push((
                                position,
                                (
                                    cuboid.half_extents.x * 2.0,
                                    cuboid.half_extents.y * 2.0,
                                ),
                                "Rectangle".to_string(),
                            ));
                        }
                    }
                }
            }
        }

        colliders
    }

    // Angular motion
    pub fn get_angular_velocity(&self, entity_id: &Uuid) -> Option<Real> {
        self.entity_to_body.get(entity_id)
            .and_then(|rb_handle| self.rigid_body_set.get(*rb_handle))
            .map(|rb| rb.angvel())
    }

    pub fn set_angular_velocity(&mut self, entity_id: &Uuid, angular_vel: Real) {
        if let Some(rb_handle) = self.entity_to_body.get(entity_id) {
            if let Some(rb) = self.rigid_body_set.get_mut(*rb_handle) {
                rb.set_angvel(angular_vel, true);
            }
        }
    }

    pub fn apply_torque(&mut self, entity_id: &Uuid, torque: Real) {
        if let Some(rb_handle) = self.entity_to_body.get(entity_id) {
            if let Some(rb) = self.rigid_body_set.get_mut(*rb_handle) {
                rb.add_torque(torque, true);
            }
        }
    }

    // Movement status
    pub fn is_moving(&self, entity_id: &Uuid) -> bool {
        if let Some(vel) = self.get_velocity(entity_id) {
            let linear_moving = vel.norm() > 0.001;
            let angular_moving = self.get_angular_velocity(entity_id)
                .map(|av| av.abs() > 0.001)
                .unwrap_or(false);
            linear_moving || angular_moving
        } else {
            false
        }
    }

    pub fn is_stable(&self, entity_id: &Uuid) -> bool {
        !self.is_moving(entity_id)
    }

    pub fn is_empty(&self) -> bool {
        self.entity_to_body.is_empty() && self.entity_to_collider.is_empty()
    }

    pub fn has_rigid_body(&self, entity_id: &Uuid) -> bool {
        self.entity_to_body.contains_key(entity_id)
    }

    pub fn has_collider(&self, entity_id: &Uuid) -> bool {
        self.entity_to_collider.contains_key(entity_id)
    }
}