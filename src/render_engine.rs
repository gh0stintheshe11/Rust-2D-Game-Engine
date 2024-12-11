use std::time::Instant;
use std::collections::HashMap;
use wgpu;
use egui;

// Layer System
#[derive(Hash, Eq, PartialEq, Clone, Copy)]
pub enum RenderLayer {
    Background,
    Game,
    UI,
    Debug,
}

// Transform System
pub struct Transform {
    pub position: (f32, f32),
    pub scale: (f32, f32),
    pub rotation: f32,
}

impl Transform {
    pub fn new(position: (f32, f32)) -> Self {
        Self {
            position,
            scale: (1.0, 1.0),
            rotation: 0.0,
        }
    }
}

// Camera System
pub struct Camera {
    pub position: (f32, f32),
    pub zoom: f32,
    pub rotation: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: (0.0, 0.0),
            zoom: 1.0,
            rotation: 0.0,
        }
    }

    pub fn transform_point(&self, point: (f32, f32)) -> (f32, f32) {
        // Apply camera transformations to a world point
        let (x, y) = point;
        let (cam_x, cam_y) = self.position;
        
        // Translate
        let x = (x - cam_x) * self.zoom;
        let y = (y - cam_y) * self.zoom;
        
        // Rotate (if needed)
        if self.rotation != 0.0 {
            let cos_r = self.rotation.cos();
            let sin_r = self.rotation.sin();
            let rx = x * cos_r - y * sin_r;
            let ry = x * sin_r + y * cos_r;
            (rx, ry)
        } else {
            (x, y)
        }
    }
}

// Sprite Sheet System
pub struct SpriteSheet {
    pub texture: egui::TextureHandle,
    pub frame_size: (u32, u32),
    pub frames: Vec<egui::Rect>,
}

impl SpriteSheet {
    pub fn new(texture: egui::TextureHandle, frame_size: (u32, u32), frames_count: usize) -> Self {
        let mut frames = Vec::with_capacity(frames_count);
        let (frame_width, frame_height) = frame_size;
        let texture_width = texture.size()[0] as u32;
        let frames_per_row = texture_width / frame_width;

        for i in 0..frames_count {
            let x = (i as u32 % frames_per_row) * frame_width;
            let y = (i as u32 / frames_per_row) * frame_height;
            frames.push(egui::Rect::from_min_size(
                egui::pos2(x as f32, y as f32),
                egui::vec2(frame_width as f32, frame_height as f32),
            ));
        }

        Self {
            texture,
            frame_size,
            frames,
        }
    }
}

// Batch Rendering System
pub struct RenderBatch {
    pub texture: egui::TextureHandle,
    pub instances: Vec<InstanceData>,
}

pub struct InstanceData {
    pub transform: Transform,
    pub color: [f32; 4],
    pub uv_rect: egui::Rect,
}

// Updated RenderObject to use new systems
pub enum RenderObject {
    Static {
        texture: egui::TextureHandle,
        transform: Transform,
    },
    Animated {
        animation: Animation,
        transform: Transform,
    },
    Sprite {
        sprite_sheet: SpriteSheet,
        current_frame: usize,
        transform: Transform,
    },
}

// Updated Scene to use layers and batching
pub struct Scene {
    pub layers: HashMap<RenderLayer, Vec<RenderObject>>,
    pub camera: Camera,
}

impl Scene {
    pub fn new() -> Self {
        let mut layers = HashMap::new();
        layers.insert(RenderLayer::Background, Vec::new());
        layers.insert(RenderLayer::Game, Vec::new());
        layers.insert(RenderLayer::UI, Vec::new());
        layers.insert(RenderLayer::Debug, Vec::new());

        Self {
            layers,
            camera: Camera::new(),
        }
    }

    pub fn add_object(&mut self, object: RenderObject, layer: RenderLayer) {
        if let Some(objects) = self.layers.get_mut(&layer) {
            objects.push(object);
        }
    }

    pub fn update(&mut self, dt: f32) {
        // Update all layers
        for (_layer, objects) in self.layers.iter_mut() {
            for object in objects {
                if let RenderObject::Animated { animation, .. } = object {
                    animation.update(dt);
                }
            }
        }
    }

    pub fn move_camera(&mut self, delta: (f32, f32)) {
        self.camera.position.0 += delta.0;
        self.camera.position.1 += delta.1;
    }

    pub fn zoom_camera(&mut self, delta: f32) {
        self.camera.zoom = (self.camera.zoom + delta).clamp(0.1, 10.0);
    }

    pub fn rotate_camera(&mut self, angle: f32) {
        self.camera.rotation += angle;
    }
}

pub struct RenderEngine {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::Adapter,
    pub texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
    pub last_frame_time: Instant,
    pub delta_time: f32,
}

pub struct Animation {
    frames: Vec<egui::TextureHandle>,
    frame_duration: f32,
    pub current_frame: usize,
    timer: f32,
    is_playing: bool,
    is_looping: bool,
}

impl Animation {
    pub fn new(frames: Vec<egui::TextureHandle>, frame_duration: f32) -> Self {
        Self {
            frames,
            frame_duration,
            current_frame: 0,
            timer: 0.0,
            is_playing: true,
            is_looping: true,
        }
    }

    pub fn update(&mut self, dt: f32) {
        if self.is_playing && !self.frames.is_empty() {
            self.timer += dt;
            if self.timer >= self.frame_duration {
                self.timer = 0.0;
                if self.current_frame + 1 < self.frames.len() {
                    self.current_frame += 1;
                } else if self.is_looping {
                    self.current_frame = 0;
                } else {
                    self.is_playing = false;
                }
            }
        }
    }

    pub fn current_frame(&self) -> Option<&egui::TextureHandle> {
        self.frames.get(self.current_frame)
    }

    pub fn play(&mut self) {
        self.is_playing = true;
    }

    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.timer = 0.0;
    }
}

impl RenderEngine {
    pub fn new() -> Self {
        // Initialize wgpu
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            flags: wgpu::InstanceFlags::empty(),
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        });

        // Create adapter
        let adapter = futures::executor::block_on(async {
            instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .unwrap()
        });

        // Create device and queue
        let (device, queue) = futures::executor::block_on(async {
            adapter.request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .unwrap()
        });

        // Create a test texture with solid color
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Test Texture"),
            size: wgpu::Extent3d {
                width: 100,
                height: 100,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING 
                | wgpu::TextureUsages::COPY_DST 
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        // Create solid red texture data
        let texture_data = vec![255u8, 0, 0, 255].repeat(100 * 100);

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &texture_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * 100),
                rows_per_image: Some(100),
            },
            wgpu::Extent3d {
                width: 100,
                height: 100,
                depth_or_array_layers: 1,
            },
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            device,
            queue,
            adapter,
            texture,
            texture_view,
            last_frame_time: Instant::now(),
            delta_time: 0.0,
        }
    }

    // Add method to update timing and handle frame updates
    pub fn update(&mut self) {
        let current_time = Instant::now();
        self.delta_time = current_time.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = current_time;
    }

    // Method to update texture with new frame data
    pub fn update_texture(&mut self, frame_data: &[u8], width: u32, height: u32) {
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            frame_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    // We might add more methods here later for game-specific rendering
}
