use rapier2d::prelude::*;

pub struct PhysicsEngine {
    pub physics_pipeline: PhysicsPipeline,
    pub gravity: Vector<Real>,
    pub integration_parameters: IntegrationParameters,
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub island_manager: IslandManager,
    pub broad_phase: Box<dyn BroadPhase>,  // Use Box<dyn BroadPhase>
    pub narrow_phase: NarrowPhase,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: QueryPipeline,
}

impl PhysicsEngine {
    pub fn new() -> Self {
        let gravity = vector![0.0, -9.81];  // Gravity pulling objects downward.
    
        PhysicsEngine {
            physics_pipeline: PhysicsPipeline::new(),
            gravity,
            integration_parameters: IntegrationParameters {
                dt: 1.0 / 60.0,  // Set the time step to 1/60th of a second
                ..Default::default()
            },
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            island_manager: IslandManager::new(),
            broad_phase: Box::new(DefaultBroadPhase::new()),  // Use DefaultBroadPhase::new()
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
        }
    }

    pub fn step(&mut self) {
        let physics_hooks = ();
        let event_handler = ();
    
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut *self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &physics_hooks,
            &event_handler,
        );
    
        // Debugging output to see if bodies are moving
        for (handle, body) in self.rigid_body_set.iter() {
            let position = body.translation();
            println!("Body handle: {:?}, Position: {:?}", handle, position);
        }
    }
    
    pub fn add_rigid_body(&mut self, position: [f32; 2], is_dynamic: bool) -> Option<RigidBodyHandle> {
        if position[0].is_nan() || position[1].is_nan() {
            return None; // Do not add invalid rigid body
        }
    
        let rigid_body = if is_dynamic {
            RigidBodyBuilder::dynamic()
                .translation(vector![position[0], position[1]])
                .can_sleep(false) // Ensure the body stays awake
                .build()
        } else {
            RigidBodyBuilder::fixed()
                .translation(vector![position[0], position[1]])
                .build()
        };
    
        // Insert the rigid body and return the handle
        Some(self.rigid_body_set.insert(rigid_body))
    }

    // Detect collisions and handle events
    pub fn handle_collisions(&self) {
        for contact_pair in self.narrow_phase.contact_pairs() {
            let collider1 = contact_pair.collider1;
            let collider2 = contact_pair.collider2;
            println!("Collision detected between {:?} and {:?}", collider1, collider2);
        }
    }
}