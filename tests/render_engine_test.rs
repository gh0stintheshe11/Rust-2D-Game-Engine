#[cfg(test)]
mod tests {
    use rust_2d_game_engine::render_engine::{RenderEngine, Camera, Transform, TextureInfo};
    use rust_2d_game_engine::ecs::Scene;
    use std::path::Path;

    #[test]
    fn test_camera_operations() {
        let mut camera = Camera::new();
        
        // Test initial state
        assert_eq!(camera.position, (0.0, 0.0));
        assert_eq!(camera.zoom, 1.0);
        
        // Test movement
        camera.move_by(10.0, 5.0);
        assert_eq!(camera.position, (10.0, 5.0));
        
        // Test zoom
        camera.zoom_by(2.0);
        assert_eq!(camera.zoom, 2.0);
        
        // Test world to screen conversion
        let screen_pos = camera.world_to_screen((15.0, 10.0));
        assert_eq!(screen_pos, ((15.0 - 10.0) * 2.0, (10.0 - 5.0) * 2.0));
        
        // Test reset
        camera.reset();
        assert_eq!(camera.position, (0.0, 0.0));
        assert_eq!(camera.zoom, 1.0);
    }

    #[test]
    fn test_render_engine_initialization() {
        let renderer = RenderEngine::new();
        assert_eq!(renderer.get_viewport_size(), (0.0, 0.0));
        assert!(renderer.texture_cache.is_empty());
    }

    #[test]
    fn test_viewport_update() {
        let mut renderer = RenderEngine::new();
        renderer.update_viewport_size(800.0, 600.0);
        assert_eq!(renderer.get_viewport_size(), (800.0, 600.0));
    }

    #[test]
    fn test_transform_operations() {
        let transform = Transform::new()
            .with_position(10.0, 20.0)
            .with_rotation(1.5)
            .with_scale(2.0, 3.0);
        
        assert_eq!(transform.position, (10.0, 20.0));
        assert_eq!(transform.rotation, 1.5);
        assert_eq!(transform.scale, (2.0, 3.0));
    }

    #[test]
    fn test_texture_cache_operations() {
        let mut renderer = RenderEngine::new();
        
        // Create a test texture info
        let texture_info = TextureInfo {
            data: vec![255; 4], // Simple 1x1 RGBA texture
            dimensions: (1, 1),
            aspect_ratio: 1.0,
        };
        
        // Test texture caching
        let test_path = Path::new("test.png");
        let texture_id = RenderEngine::path_to_uuid(test_path);
        renderer.texture_cache.insert(texture_id, texture_info);
        
        // Test cache retrieval
        assert!(renderer.get_texture_info(&texture_id).is_some());
        
        // Test cache clearing
        renderer.clear_cache();
        assert!(renderer.texture_cache.is_empty());
    }

    #[test]
    fn test_grid_lines_generation() {
        let mut renderer = RenderEngine::new();
        renderer.update_viewport_size(800.0, 600.0);
        
        let grid_lines = renderer.get_grid_lines();
        assert!(!grid_lines.is_empty(), "Grid lines should be generated");
    }

    #[test]
    fn test_camera_bounds() {
        let mut renderer = RenderEngine::new();
        let mut scene = Scene::new("test_scene").unwrap();
        
        // Create a camera entity
        let camera_id = scene.create_camera("test_camera").unwrap();
        scene.default_camera = Some(camera_id);
        
        let bounds = renderer.get_game_camera_bounds(&scene);
        assert!(!bounds.is_empty(), "Camera bounds should be generated");
    }

    #[test]
    fn test_memory_usage() {
        let mut renderer = RenderEngine::new();
        
        // Add a test texture to the cache
        let texture_info = TextureInfo {
            data: vec![255; 1024], // 1KB of data
            dimensions: (16, 16),
            aspect_ratio: 1.0,
        };
        
        let test_path = Path::new("test.png");
        let texture_id = RenderEngine::path_to_uuid(test_path);
        renderer.texture_cache.insert(texture_id, texture_info);
        
        assert_eq!(renderer.get_memory_usage(), 1024);
    }
}
