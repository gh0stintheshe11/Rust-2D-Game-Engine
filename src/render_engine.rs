use image::GenericImageView;
use std::collections::HashMap;
use uuid::Uuid;
use crate::ecs::{Scene, ResourceType};

pub struct Camera {
    pub position: (f32, f32),
    pub zoom: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: (0.0, 0.0),
            zoom: 1.0,
        }
    }

    pub fn move_by(&mut self, dx: f32, dy: f32) {
        self.position.0 += dx;
        self.position.1 += dy;
    }

    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(0.1, 10.0);
    }

    pub fn world_to_screen(&self, world_pos: (f32, f32)) -> (f32, f32) {
        (
            (world_pos.0 - self.position.0) * self.zoom,
            (world_pos.1 - self.position.1) * self.zoom,
        )
    }

    pub fn reset(&mut self) {
        self.position = (0.0, 0.0);
        self.zoom = 1.0;
    }
}

#[derive(Debug)]
pub struct TextureInfo {
    data: Vec<u8>,
    dimensions: (u32, u32), // Original width and height in pixels
    aspect_ratio: f32,
}

// Layers for rendering order
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderLayer {
    Background = 0,
    Game = 1,
    UI = 2,
    Debug = 3,
}

// Animation support with enhanced controls
#[derive(Debug)]
pub struct Animation {
    frames: Vec<TextureInfo>,
    frame_duration: f32,
    current_frame: usize,
    elapsed_time: f32,
    is_playing: bool,
    is_looping: bool,
    playback_speed: f32,
}

impl Animation {
    pub fn new(frames: Vec<TextureInfo>, frame_duration: f32) -> Self {
        Self {
            frames,
            frame_duration,
            current_frame: 0,
            elapsed_time: 0.0,
            is_playing: true,
            is_looping: true,
            playback_speed: 1.0,
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        if !self.is_playing || self.frames.is_empty() {
            return;
        }

        self.elapsed_time += delta_time * self.playback_speed;

        if self.elapsed_time >= self.frame_duration {
            let next_frame = self.current_frame + 1;

            if next_frame >= self.frames.len() {
                if self.is_looping {
                    self.current_frame = 0;
                } else {
                    self.is_playing = false;
                    self.current_frame = self.frames.len() - 1; // Stay on last frame
                }
            } else {
                self.current_frame = next_frame;
            }

            self.elapsed_time = 0.0;
        }
    }

    // Control methods
    pub fn play(&mut self) {
        self.is_playing = true;
    }

    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    pub fn stop(&mut self) {
        self.is_playing = false;
        self.current_frame = 0;
        self.elapsed_time = 0.0;
    }

    pub fn set_looping(&mut self, looping: bool) {
        self.is_looping = looping;
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.playback_speed = speed.max(0.0); // Prevent negative speed
    }

    pub fn set_frame(&mut self, frame: usize) {
        if frame < self.frames.len() {
            self.current_frame = frame;
            self.elapsed_time = 0.0;
        }
    }

    // State queries
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    pub fn is_finished(&self) -> bool {
        !self.is_looping && self.current_frame == self.frames.len() - 1
    }

    pub fn get_current_frame(&self) -> Option<&TextureInfo> {
        self.frames.get(self.current_frame)
    }

    pub fn get_frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn get_current_frame_index(&self) -> usize {
        self.current_frame
    }

    pub fn get_progress(&self) -> f32 {
        if self.frames.is_empty() {
            return 0.0;
        }
        self.current_frame as f32 / (self.frames.len() - 1) as f32
    }
}

pub struct RenderEngine {
    viewport_size: (f32, f32),
    last_frame_time: std::time::Instant,
    scene_texture_cache: HashMap<Uuid, HashMap<Uuid, TextureInfo>>, // scene_id -> (resource_id -> texture)
    direct_textures: HashMap<String, TextureInfo>, // For UI/preview
    pub camera: Camera,
}

impl RenderEngine {
    pub fn new() -> Self {
        Self {
            viewport_size: (0.0, 0.0),
            last_frame_time: std::time::Instant::now(),
            scene_texture_cache: HashMap::new(),
            direct_textures: HashMap::new(),
            camera: Camera::new(),
        }
    }

    // Core loading functionality
    fn load_texture_from_path(&self, path: &str) -> Result<TextureInfo, String> {
        let img = image::open(path)
            .map_err(|e| format!("Failed to load image {}: {}", path, e))?;

        let dimensions = img.dimensions();
        let aspect_ratio = dimensions.0 as f32 / dimensions.1 as f32;
        let rgba = img.to_rgba8();

        Ok(TextureInfo {
            data: rgba.to_vec(),
            dimensions,
            aspect_ratio,
        })
    }

    // For scene resources
    pub fn load_scene_textures(&mut self, scene: &Scene) -> Result<(), String> {
        let mut scene_cache = HashMap::new();
        
        for (resource_id, resource) in scene.resources.iter()
            .filter(|(_, r)| r.resource_type == ResourceType::Image)
        {
            let texture = self.load_texture_from_path(&resource.file_path)?;
            scene_cache.insert(*resource_id, texture);
        }

        self.scene_texture_cache.insert(scene.id, scene_cache);
        Ok(())
    }

    // For UI/preview (direct usage)
    pub fn load_direct_texture(&mut self, path: &str) -> Result<(), String> {
        if !self.direct_textures.contains_key(path) {
            let texture = self.load_texture_from_path(path)?;
            self.direct_textures.insert(path.to_string(), texture);
        }
        Ok(())
    }

    // Get texture data methods
    pub fn get_scene_texture(&self, scene_id: Uuid, resource_id: Uuid) -> Option<(&Vec<u8>, (u32, u32))> {
        self.scene_texture_cache.get(&scene_id)?
            .get(&resource_id)
            .map(|info| (&info.data, info.dimensions))
    }

    pub fn get_direct_texture(&self, path: &str) -> Option<(&Vec<u8>, (u32, u32))> {
        self.direct_textures.get(path)
            .map(|info| (&info.data, info.dimensions))
    }

    // Memory management
    pub fn cleanup_direct_textures(&mut self) {
        self.direct_textures.clear();
    }

    // Keep existing methods unchanged
    pub fn update_viewport_size(&mut self, width: f32, height: f32) {
        self.viewport_size = (width, height);
    }

    pub fn get_viewport_size(&self) -> (f32, f32) {
        self.viewport_size
    }

    // Modified to use scene texture cache
    pub fn render(
        &mut self,
        scene: &Scene,
    ) -> Vec<(Uuid, (f32, f32), (f32, f32), RenderLayer)> {
        let mut render_queue = Vec::new();

        let scene_textures = if let Some(textures) = self.scene_texture_cache.get(&scene.id) {
            textures
        } else {
            return render_queue;
        };

        for (_, entity) in &scene.entities {
            // Get transform components
            let transform = Transform {
                position: entity
                    .get_attribute_by_name("position")
                    .and_then(|attr| {
                        if let crate::ecs::AttributeValue::Vector2(x, y) = attr.value {
                            Some((x, y))
                        } else {
                            None
                        }
                    })
                    .unwrap_or((0.0, 0.0)),

                rotation: entity
                    .get_attribute_by_name("rotation")
                    .and_then(|attr| {
                        if let crate::ecs::AttributeValue::Float(r) = attr.value {
                            Some(r)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0.0),

                scale: entity
                    .get_attribute_by_name("scale")
                    .and_then(|attr| {
                        if let crate::ecs::AttributeValue::Vector2(sx, sy) = attr.value {
                            Some((sx, sy))
                        } else {
                            None
                        }
                    })
                    .unwrap_or((1.0, 1.0)),
            };

            let sprite_resource = entity.get_attribute_by_name("sprite").and_then(|attr| {
                if let crate::ecs::AttributeValue::String(resource_id) = &attr.value {
                    Uuid::parse_str(&resource_id).ok()
                } else {
                    None
                }
            });

            let layer = entity
                .get_attribute_by_name("layer")
                .and_then(|attr| {
                    if let crate::ecs::AttributeValue::Integer(layer) = attr.value {
                        Some(match layer {
                            0 => RenderLayer::Background,
                            1 => RenderLayer::Game,
                            2 => RenderLayer::UI,
                            3 => RenderLayer::Debug,
                            _ => RenderLayer::Game,
                        })
                    } else {
                        None
                    }
                })
                .unwrap_or(RenderLayer::Game);

            if let Some(sprite_id) = sprite_resource {
                let screen_pos = self.camera.world_to_screen(transform.position);

                if let Some(texture_info) = scene_textures.get(&sprite_id) {
                    // Apply transform scale to the texture dimensions
                    let width =
                        texture_info.dimensions.0 as f32 * self.camera.zoom * transform.scale.0;
                    let height =
                        texture_info.dimensions.1 as f32 * self.camera.zoom * transform.scale.1;

                    // Basic visibility check
                    if screen_pos.0 + width >= 0.0
                        && screen_pos.0 <= self.viewport_size.0
                        && screen_pos.1 + height >= 0.0
                        && screen_pos.1 <= self.viewport_size.1
                    {
                        render_queue.push((sprite_id, screen_pos, (width, height), layer));
                    }
                }
            }
        }

        render_queue.sort_by_key(|(_, _, _, layer)| *layer);
        render_queue
    }
}

// Transform component for positioning, scaling, and rotating
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub position: (f32, f32),
    pub rotation: f32, // In radians
    pub scale: (f32, f32),
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: (0.0, 0.0),
            rotation: 0.0,
            scale: (1.0, 1.0),
        }
    }

    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.position = (x, y);
        self
    }

    pub fn with_rotation(mut self, angle: f32) -> Self {
        self.rotation = angle;
        self
    }

    pub fn with_scale(mut self, sx: f32, sy: f32) -> Self {
        self.scale = (sx, sy);
        self
    }

    // Helper for uniform scaling
    pub fn with_uniform_scale(mut self, scale: f32) -> Self {
        self.scale = (scale, scale);
        self
    }
}
