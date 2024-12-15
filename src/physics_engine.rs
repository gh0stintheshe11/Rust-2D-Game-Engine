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
}

impl PhysicsEngine {
    pub fn new() -> Self {
        Self {
            // Default gravity points downward (-Y direction)
            gravity: vector![0.0, -9.81],

            // Physics runs at 60Hz (60 updates per second)
            integration_parameters: IntegrationParameters {
                dt: 1.0 / 60.0,
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
        }
    }

    fn create_collider(&self, entity: &Entity, scene: &Scene, density: f32, friction: f32, restitution: f32) -> Collider {
        // Get sprite resource ID
        let sprite_id = entity.get_attribute_by_name("sprite")
            .and_then(|attr| if let AttributeValue::String(id) = &attr.value {
                Uuid::parse_str(id).ok()
            } else { None });

        // Get sprite dimensions from resource
        let collider_builder = if let Some(sprite_id) = sprite_id {
            if let Some(resource) = scene.get_resource(sprite_id) {
                // Get image dimensions
                if let Ok(img) = image::open(&resource.file_path) {
                    let (width, height) = img.dimensions();
                    
                    // If width and height are similar, use circle
                    if (width as f32 / height as f32).abs() > 0.9 
                       && (width as f32 / height as f32).abs() < 1.1 {
                        ColliderBuilder::ball(width as f32 / 2.0)
                    } else {
                        // Otherwise use box
                        ColliderBuilder::cuboid(width as f32 / 2.0, height as f32 / 2.0)
                    }
                } else {
                    ColliderBuilder::ball(0.5) // Default if can't load image
                }
            } else {
                ColliderBuilder::ball(0.5) // Default if resource not found
            }
        } else {
            ColliderBuilder::ball(0.5) // Default if no sprite
        };

        // Add physics properties
        collider_builder
            .density(density)
            .friction(friction)
            .restitution(restitution)
            .build()
    }

    pub fn add_entity(&mut self, entity: &Entity, scene: &Scene) {
        // Get physics properties from entity attributes
        let position = if let Some(pos_attr) = entity.get_attribute_by_name("position") {
            if let AttributeValue::Vector2(x, y) = pos_attr.value {
                vector![x, y]
            } else {
                vector![0.0, 0.0]
            }
        } else {
            vector![0.0, 0.0]
        };

        let is_movable = entity.get_attribute_by_name("is_movable")
            .and_then(|attr| if let AttributeValue::Boolean(v) = attr.value { Some(v) } else { None })
            .unwrap_or(false);

        let affected_by_gravity = entity.get_attribute_by_name("has_gravity")
            .and_then(|attr| if let AttributeValue::Boolean(v) = attr.value { Some(v) } else { None })
            .unwrap_or(false);

        let has_collision = entity.get_attribute_by_name("has_collision")
            .and_then(|attr| if let AttributeValue::Boolean(v) = attr.value { Some(v) } else { None })
            .unwrap_or(true);

        let friction = entity.get_attribute_by_name("friction")
            .and_then(|attr| if let AttributeValue::Float(v) = attr.value { Some(v) } else { None })
            .unwrap_or(0.5);

        let restitution = entity.get_attribute_by_name("restitution")
            .and_then(|attr| if let AttributeValue::Float(v) = attr.value { Some(v) } else { None })
            .unwrap_or(0.0);

        let density = entity.get_attribute_by_name("density")
            .and_then(|attr| if let AttributeValue::Float(v) = attr.value { Some(v) } else { None })
            .unwrap_or(1.0);

        let can_rotate = entity.get_attribute_by_name("can_rotate")
            .and_then(|attr| if let AttributeValue::Boolean(v) = attr.value { Some(v) } else { None })
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
            let collider = self.create_collider(entity, scene, density, friction, restitution);
            let collider_handle = self.collider_set
                .insert_with_parent(collider, rb_handle, &mut self.rigid_body_set);
            self.entity_to_collider.insert(entity.id, collider_handle);
        }

        self.entity_to_body.insert(entity.id, rb_handle);
    }

    pub fn remove_entity(&mut self, entity_id: Uuid) {
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

    pub fn step(&mut self, scene: &mut Scene) {
        // Process custom gravity fields
        for (_, entity1) in &scene.entities {
            if let Some(creates_gravity) = entity1.get_attribute_by_name("creates_gravity") {
                if let AttributeValue::Boolean(true) = creates_gravity.value {
                    let pos1 = if let Some(pos) = entity1.get_attribute_by_name("position") {
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

                        if let Some(affected_by_gravity) = entity2.get_attribute_by_name("has_gravity") {
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

        // Update entity positions
        for (_, entity) in &mut scene.entities {
            if let Some(rb_handle) = self.entity_to_body.get(&entity.id) {
                if let Some(rb) = self.rigid_body_set.get(*rb_handle) {
                    let position = rb.translation();
                    if let Some(pos_attr) = entity.get_attribute_by_name("position") {
                        entity.modify_attribute(
                            pos_attr.id,
                            None,
                            None,
                            Some(AttributeValue::Vector2(position.x, position.y))
                        );
                    }
                }
            }
        }
    }

    pub fn load_scene(&mut self, scene: &Scene) {
        for (_, entity) in &scene.entities {
            self.add_entity(entity, scene);
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
}