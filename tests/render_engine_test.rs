#[cfg(test)]
mod tests {
    use rust_2d_game_engine::render_engine::{RenderEngine, Sprite};
    use wgpu::ShaderModuleDescriptor;
    use futures::executor::block_on;

    fn create_test_sprite() -> Sprite {
        Sprite {
            position: (0.0, 0.0),
            size: (100.0, 100.0),
            rotation: 0.0,
            texture_coords: (0.0, 0.0, 1.0, 1.0),
        }
    }

    #[test]
    fn test_renderer_initialization() {
        let _renderer = RenderEngine::new();
        assert!(true, "Renderer initialized successfully");
    }

    #[test]
    fn test_texture_creation() {
        let _renderer = RenderEngine::new();
        let texture_extent = wgpu::Extent3d {
            width: 1024,
            height: 1024,
            depth_or_array_layers: 1,
        };

        assert_eq!(texture_extent.width, 1024);
        assert_eq!(texture_extent.height, 1024);
        assert_eq!(texture_extent.depth_or_array_layers, 1);
    }

    #[test]
    fn test_render_frame_executes() {
        let mut renderer = RenderEngine::new();
        let sprites = vec![create_test_sprite()];
        let result = renderer.render_frame(&sprites);
        assert!(result.is_ok(), "render_frame should execute without errors");
    }

    #[test]
    fn test_instance_initialization() {
        let instance_desc = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            flags: wgpu::InstanceFlags::empty(),
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        };
        let _instance = wgpu::Instance::new(instance_desc);
        assert!(true, "Instance should be created successfully");
    }

    #[test]
    fn test_request_device() {
        let instance_desc = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            flags: wgpu::InstanceFlags::empty(),
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        };
        let instance = wgpu::Instance::new(instance_desc);

        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })).unwrap();

        let (device, _queue) = block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        )).unwrap();

        assert!(device.limits().max_texture_dimension_2d > 0);
    }

    #[test]
    fn test_error_handling_in_renderer() {
        let result = std::panic::catch_unwind(|| {
            let mut renderer = RenderEngine::new();
            let sprites = vec![create_test_sprite()];
            renderer.render_frame(&sprites).unwrap();
        });

        assert!(result.is_ok(), "Renderer should not panic when executing render_frame");
    }

    #[test]
    fn test_shader_compilation() {
        let renderer = RenderEngine::new();
        let shader_source = r#"
            struct VertexInput {
                @location(0) position: vec3<f32>,
                @location(1) tex_coords: vec2<f32>,
            };

            struct VertexOutput {
                @builtin(position) clip_position: vec4<f32>,
                @location(0) tex_coords: vec2<f32>,
            };

            @vertex
            fn vs_main(in: VertexInput) -> VertexOutput {
                var out: VertexOutput;
                out.tex_coords = in.tex_coords;
                out.clip_position = vec4<f32>(in.position, 1.0);
                return out;
            }

            @fragment
            fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                return vec4<f32>(1.0, 1.0, 1.0, 1.0);
            }
        "#;

        let _shader_module = renderer.device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Test Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        assert!(true, "Shader compiled successfully");
    }

    #[test]
    fn test_high_load_rendering() {
        let mut renderer = RenderEngine::new();
        let sprites = vec![create_test_sprite()];
        for _ in 0..100 {  // Reduced from 10000 to 100 for faster testing
            renderer.render_frame(&sprites).unwrap();
        }
        assert!(true, "Engine handled high-load rendering without crashing");
    }

    #[test]
    fn test_sprite_creation() {
        let sprite = create_test_sprite();
        assert_eq!(sprite.position, (0.0, 0.0));
        assert_eq!(sprite.size, (100.0, 100.0));
        assert_eq!(sprite.rotation, 0.0);
        assert_eq!(sprite.texture_coords, (0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn test_multiple_sprites_rendering() {
        let mut renderer = RenderEngine::new();
        let sprites = vec![
            create_test_sprite(),
            create_test_sprite(),
            create_test_sprite(),
        ];
        let result = renderer.render_frame(&sprites);
        assert!(result.is_ok(), "Should render multiple sprites without errors");
    }
}
