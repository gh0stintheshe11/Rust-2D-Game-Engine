#[cfg(test)]
mod tests {
    use rust_2d_game_engine::physics_engine::PhysicsEngine;
    
    #[test]
    fn test_initialization() {
        // Initialize the physics engine
        let physics_engine = PhysicsEngine::new();
        
        // Check that gravity, bodies, etc., are initialized correctly
        assert_eq!(physics_engine.gravity.x, 0.0, "Gravity x-component is incorrect.");
        assert_eq!(physics_engine.gravity.y, -9.81, "Gravity y-component is incorrect.");
        assert_eq!(physics_engine.rigid_body_set.len(), 0, "Rigid body set should be empty upon initialization.");
    }

    #[test]
    fn test_add_rigid_body_dynamic() {
        let mut physics_engine = PhysicsEngine::new();
        let initial_body_count = physics_engine.rigid_body_set.len();
    
        // Add a dynamic rigid body and store its handle
        let handle = physics_engine.add_rigid_body([1.0, 2.0], true)
            .expect("Failed to add dynamic rigid body");
    
        // Check that the number of bodies increased
        assert_eq!(physics_engine.rigid_body_set.len(), initial_body_count + 1, "Dynamic rigid body was not added.");
    
        // Use the handle to retrieve the body
        let body = physics_engine.rigid_body_set.get(handle)
            .expect("Failed to retrieve the rigid body from the set");
        assert!(body.is_dynamic(), "Rigid body should be dynamic.");
        assert_eq!(body.translation().x, 1.0, "Rigid body x position is incorrect.");
        assert_eq!(body.translation().y, 2.0, "Rigid body y position is incorrect.");
    }

    #[test]
    fn test_add_rigid_body_static() {
        let mut physics_engine = PhysicsEngine::new();
        let initial_body_count = physics_engine.rigid_body_set.len();
        
        // Add a static rigid body and handle errors if the body could not be added
        let handle = physics_engine.add_rigid_body([3.0, 4.0], false)
            .expect("Failed to add static rigid body");
        
        // Check that the number of bodies increased
        assert_eq!(physics_engine.rigid_body_set.len(), initial_body_count + 1, "Static rigid body was not added.");
        
        // Retrieve the body from the set
        let body = physics_engine.rigid_body_set.get(handle)
            .expect("Failed to retrieve the rigid body");
        assert!(body.is_fixed(), "Rigid body should be static.");
        assert_eq!(body.translation().x, 3.0, "Rigid body x position is incorrect.");
        assert_eq!(body.translation().y, 4.0, "Rigid body y position is incorrect.");
    }

    #[test]
    fn test_simulation_step() {
        let mut physics_engine = PhysicsEngine::new();
    
        // Add a dynamic body at a non-zero height
        let handle = physics_engine.add_rigid_body([0.0, 10.0], true).unwrap();
    
        // Run the simulation for multiple steps
        for i in 0..100 {  // Increase the number of simulation steps
            physics_engine.step();
            let body = physics_engine.rigid_body_set.get(handle).unwrap();
            println!("Step {}: Body y position = {}", i, body.translation().y);
    
            // Ensure that after some steps, the body falls below its starting point
            if i > 10 {
                assert!(body.translation().y < 10.0, "Body did not fall after simulation steps.");
            }
        }
    }

    #[test]
    fn test_handle_collisions() {
        let mut physics_engine = PhysicsEngine::new();
    
        // Add two rigid bodies for collision
        let body1 = physics_engine.add_rigid_body([0.0, 0.0], false).expect("Failed to add body1");
        let _body2 = physics_engine.add_rigid_body([0.5, 0.5], true).expect("Failed to add body2");
    
        // Simulate physics step to check for collisions
        physics_engine.step();
    
        // Use the handle to retrieve the body
        let body = physics_engine.rigid_body_set.get(body1).expect("Failed to retrieve body1");
        assert_eq!(body.translation().x, 0.0, "Body1 x position is incorrect.");
    }

    #[test]
    fn test_add_invalid_rigid_body() {
        let mut physics_engine = PhysicsEngine::new();
        
        // Try adding a body with an invalid position (e.g., NaN)
        physics_engine.add_rigid_body([f32::NAN, f32::NAN], true);
        
        // Check that the body was not added (optional check based on how you handle invalid input)
        assert_eq!(physics_engine.rigid_body_set.len(), 0, "Invalid rigid body should not be added.");
    }

    #[test]
    fn test_dynamic_body_insertion() {
        let mut physics_engine = PhysicsEngine::new();
        let initial_body_count = physics_engine.rigid_body_set.len();
    
        // Add a dynamic rigid body and store its handle
        let handle = physics_engine.add_rigid_body([1.0, 2.0], true).expect("Failed to add dynamic body");
    
        // Check that the number of bodies increased
        assert_eq!(physics_engine.rigid_body_set.len(), initial_body_count + 1, "Dynamic rigid body was not added.");
    
        // Use the handle to retrieve the body
        let body = physics_engine.rigid_body_set.get(handle).expect("Failed to retrieve dynamic body");
        assert!(body.is_dynamic(), "Rigid body should be dynamic.");
        assert_eq!(body.translation().x, 1.0, "Rigid body x position is incorrect.");
        assert_eq!(body.translation().y, 2.0, "Rigid body y position is incorrect.");
    }

    #[test]
    fn test_static_body_insertion() {
        let mut physics_engine = PhysicsEngine::new();
        let initial_body_count = physics_engine.rigid_body_set.len();
    
        // Add a static rigid body and store its handle
        let handle = physics_engine.add_rigid_body([0.0, 0.0], false).expect("Failed to add static body");
    
        // Check that the number of bodies increased
        assert_eq!(physics_engine.rigid_body_set.len(), initial_body_count + 1, "Static rigid body was not added.");
    
        // Use the handle to retrieve the body
        let body = physics_engine.rigid_body_set.get(handle).expect("Failed to retrieve static body");
        assert!(!body.is_dynamic(), "Rigid body should be static.");
        assert_eq!(body.translation().x, 0.0, "Rigid body x position is incorrect.");
        assert_eq!(body.translation().y, 0.0, "Rigid body y position is incorrect.");
    }
}