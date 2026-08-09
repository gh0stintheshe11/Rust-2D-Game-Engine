use crate::ecs::{AttributeValue, Scene};
use image::GenericImageView;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

mod camera;
mod transform;

pub use camera::Camera;
pub use transform::Transform;

/// One sprite draw command in screen space.
#[derive(Debug, Clone)]
pub struct RenderQueueEntry {
    pub entity_id: Uuid,
    pub texture_id: Uuid,
    pub screen_pos: (f32, f32),
    pub screen_size: (f32, f32),
    /// Rotation around the sprite center, in radians (entity attribute is in degrees).
    pub rotation: f32,
    pub z: f32,
}

/// Paint a textured rect, optionally rotated around its center.
pub fn paint_sprite(
    painter: &egui::Painter,
    texture_id: egui::TextureId,
    rect: egui::Rect,
    rotation: f32,
) {
    if rotation == 0.0 {
        painter.image(
            texture_id,
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        return;
    }

    let center = rect.center();
    let (sin, cos) = rotation.sin_cos();
    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ];
    let uvs = [
        egui::pos2(0.0, 0.0),
        egui::pos2(1.0, 0.0),
        egui::pos2(1.0, 1.0),
        egui::pos2(0.0, 1.0),
    ];

    let mut mesh = egui::Mesh::with_texture(texture_id);
    for (corner, uv) in corners.iter().zip(uvs) {
        let rel = *corner - center;
        let rotated = egui::vec2(rel.x * cos - rel.y * sin, rel.x * sin + rel.y * cos);
        mesh.vertices.push(egui::epaint::Vertex {
            pos: center + rotated,
            uv,
            color: egui::Color32::WHITE,
        });
    }
    mesh.indices.extend([0, 1, 2, 0, 2, 3]);
    painter.add(mesh);
}

/// One collider debug shape: screen position, screen size, shape name.
pub type ColliderRenderData = ((f32, f32), (f32, f32), String);

#[derive(Debug, Clone)]
pub struct TextureInfo {
    pub data: Vec<u8>,
    pub dimensions: (u32, u32), // Original width and height in pixels
    pub aspect_ratio: f32,
}

#[derive(Clone)]
pub struct RenderEngine {
    viewport_size: (f32, f32),
    pub texture_cache: HashMap<Uuid, TextureInfo>,
    // GPU-side textures, uploaded once per texture and reused every frame.
    // TextureHandle is an Arc internally, so cloning the engine shares them.
    egui_textures: HashMap<Uuid, egui::TextureHandle>,
    pub camera: Camera,
}

impl Default for RenderEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderEngine {
    // Generate deterministic UUID from path
    pub fn path_to_uuid(path: &Path) -> Uuid {
        let mut hasher = Sha256::new();
        hasher.update(path.to_string_lossy().as_bytes());
        let result = hasher.finalize();

        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&result[..16]);
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;

        Uuid::from_bytes(bytes)
    }

    // Load texture and return its ID
    fn load_texture(&mut self, path: &Path) -> Result<Uuid, String> {
        let texture_id = Self::path_to_uuid(path);

        if self.texture_cache.contains_key(&texture_id) {
            return Ok(texture_id);
        }

        let texture = self.load_texture_from_path(path)?;
        self.texture_cache.insert(texture_id, texture);

        Ok(texture_id)
    }

    // Get texture data using path
    pub fn get_texture(&self, path: &Path) -> Option<(&Vec<u8>, (u32, u32))> {
        let texture_id = Self::path_to_uuid(path);
        self.texture_cache
            .get(&texture_id)
            .map(|info| (&info.data, info.dimensions))
    }

    // Core loading functionality
    fn load_texture_from_path(&self, path: &Path) -> Result<TextureInfo, String> {
        let img =
            image::open(path).map_err(|e| format!("Failed to load image {:?}: {}", path, e))?;

        let dimensions = img.dimensions();
        let aspect_ratio = dimensions.0 as f32 / dimensions.1 as f32;
        let rgba = img.to_rgba8();

        Ok(TextureInfo {
            data: rgba.to_vec(),
            dimensions,
            aspect_ratio,
        })
    }

    // Modified render method to use z coordinate for ordering
    pub fn render(&mut self, scene: &Scene) -> Vec<RenderQueueEntry> {
        let mut render_queue = Vec::new();

        for (entity_id, entity) in &scene.entities {
            if let Ok(image_path) = entity.get_image(0) {
                let texture_id = Self::path_to_uuid(Path::new(image_path));

                if !self.texture_cache.contains_key(&texture_id)
                    && self.load_texture(Path::new(image_path)).is_ok()
                {
                    crate::logger::LOGGER
                        .debug(format!("Loaded texture: {}", image_path.to_string_lossy()));
                }

                // Get position including z coordinate
                let x = entity.get_x();
                let y = entity.get_y();
                let z = entity.get_z();

                let transform = Transform {
                    position: (x, y),
                    rotation: entity
                        .get_attribute_by_name("rotation")
                        .and_then(|attr| match attr.value {
                            AttributeValue::Float(r) => Ok(r),
                            _ => Err("Invalid rotation attribute type".to_string()),
                        })
                        .unwrap_or(0.0)
                        .to_radians(),

                    scale: entity
                        .get_attribute_by_name("scale")
                        .and_then(|attr| match attr.value {
                            AttributeValue::Vector2(sx, sy) => Ok((sx, sy)),
                            _ => Err("Invalid scale attribute type".to_string()),
                        })
                        .unwrap_or((1.0, 1.0)),
                };

                if let Some(texture_info) = self.texture_cache.get(&texture_id) {
                    let screen_pos = self.camera.world_to_screen(transform.position);
                    let width =
                        texture_info.dimensions.0 as f32 * self.camera.zoom * transform.scale.0;
                    let height =
                        texture_info.dimensions.1 as f32 * self.camera.zoom * transform.scale.1;

                    // Viewport culling
                    if screen_pos.0 <= self.viewport_size.0
                        && screen_pos.0 + width >= 0.0
                        && screen_pos.1 <= self.viewport_size.1
                        && screen_pos.1 + height >= 0.0
                    {
                        render_queue.push(RenderQueueEntry {
                            entity_id: *entity_id,
                            texture_id,
                            screen_pos,
                            screen_size: (width, height),
                            rotation: transform.rotation,
                            z, // Use z coordinate directly for ordering
                        });
                    }
                }
            }
        }

        // Sort by z coordinate (lower z values are rendered first)
        render_queue.sort_by(|a, b| a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal));
        render_queue
    }

    // collider_data:
    // - (f32, f32): The world coordinate of the collider (x, y).
    // - (f32, f32): The size of the collider in world coordinate (width, height).
    // - String: The shape of the collider (e.g., "Circle", "Rectangle").
    pub fn render_colliders(
        &mut self,
        collider_data: &[crate::physics_engine::ColliderData],
    ) -> Vec<ColliderRenderData> {
        let mut render_queue = Vec::new();

        for &(world_position, world_size, ref shape) in collider_data {
            // Transform position to screen space
            let screen_position = self.camera.world_to_screen(world_position);

            // Adjust size based on zoom level
            let screen_size = (
                world_size.0 * self.camera.zoom,
                world_size.1 * self.camera.zoom,
            );

            // Perform viewport culling
            if screen_position.0 + screen_size.0 >= 0.0
                && screen_position.0 <= self.viewport_size.0
                && screen_position.1 + screen_size.1 >= 0.0
                && screen_position.1 <= self.viewport_size.1
            {
                render_queue.push((screen_position, screen_size, shape.clone()));
            }
        }

        render_queue
    }

    pub fn new() -> Self {
        Self {
            viewport_size: (0.0, 0.0),
            texture_cache: HashMap::new(),
            egui_textures: HashMap::new(),
            camera: Camera::new(),
        }
    }

    /// Get (or lazily upload) the GPU texture for a cached texture ID.
    /// The upload happens only once; subsequent calls reuse the handle.
    pub fn get_egui_texture(
        &mut self,
        ctx: &egui::Context,
        texture_id: Uuid,
    ) -> Option<egui::TextureHandle> {
        if let Some(handle) = self.egui_textures.get(&texture_id) {
            return Some(handle.clone());
        }

        let info = self.texture_cache.get(&texture_id)?;
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [info.dimensions.0 as usize, info.dimensions.1 as usize],
            &info.data,
        );
        let handle = ctx.load_texture(format!("texture_{}", texture_id), image, Default::default());
        self.egui_textures.insert(texture_id, handle.clone());
        Some(handle)
    }

    // Memory management
    pub fn cleanup_direct_textures(&mut self) {
        self.texture_cache.clear();
        self.egui_textures.clear();
    }

    // Keep existing methods unchanged
    pub fn update_viewport_size(&mut self, width: f32, height: f32) {
        self.viewport_size = (width, height);
    }

    pub fn get_viewport_size(&self) -> (f32, f32) {
        self.viewport_size
    }

    // Full cleanup including camera reset
    pub fn cleanup(&mut self) {
        self.texture_cache.clear();
        self.egui_textures.clear();
        self.camera.reset();
    }

    // Remove specific texture
    pub fn unload_texture(&mut self, path: &Path) {
        let texture_id = Self::path_to_uuid(path);
        self.texture_cache.remove(&texture_id);
        self.egui_textures.remove(&texture_id);
    }

    // Just clear caches
    pub fn clear_cache(&mut self) {
        self.texture_cache.clear();
        self.egui_textures.clear();
    }

    // Monitor memory usage
    pub fn get_memory_usage(&self) -> usize {
        self.texture_cache.values().map(|tex| tex.data.len()).sum()
    }

    // Add this public method
    pub fn get_texture_info(&self, texture_id: &Uuid) -> Option<&TextureInfo> {
        self.texture_cache.get(texture_id)
    }

    // Add this new method to draw grid
    /// Grid lines in screen space, aligned to world coordinates so the grid
    /// sticks to the world at any camera position and zoom. Spacing adapts
    /// (powers of two) so lines never get denser than ~24px on screen.
    pub fn get_grid_lines(&self) -> Vec<((f32, f32), (f32, f32))> {
        let mut lines = Vec::new();
        let zoom = self.camera.zoom.max(0.0001);

        // Base cell of 32 world units, coarsened while zoomed out
        let mut spacing = 32.0_f32;
        while spacing * zoom < 24.0 {
            spacing *= 2.0;
        }

        let (width, height) = self.viewport_size;
        let world_min = self.camera.position;
        let world_max = (world_min.0 + width / zoom, world_min.1 + height / zoom);

        let mut x = (world_min.0 / spacing).floor() * spacing;
        while x <= world_max.0 {
            let sx = (x - world_min.0) * zoom;
            lines.push(((sx, 0.0), (sx, height)));
            x += spacing;
        }

        let mut y = (world_min.1 / spacing).floor() * spacing;
        while y <= world_max.1 {
            let sy = (y - world_min.1) * zoom;
            lines.push(((0.0, sy), (width, sy)));
            y += spacing;
        }

        lines
    }

    // Add this method to get game camera bounds
    pub fn get_game_camera_bounds(&self, scene: &Scene) -> Vec<((f32, f32), (f32, f32))> {
        let mut lines = Vec::new();

        if let Some(camera_id) = scene.default_camera {
            if let Ok(camera_entity) = scene.get_entity(camera_id) {
                let x = camera_entity.get_x();
                let y = camera_entity.get_y();
                let width = camera_entity.get_camera_width();
                let height = camera_entity.get_camera_height();

                // Convert game camera bounds to screen space using editor camera
                let top_left = self
                    .camera
                    .world_to_screen((x - width / 2.0, y - height / 2.0));
                let top_right = self
                    .camera
                    .world_to_screen((x + width / 2.0, y - height / 2.0));
                let bottom_left = self
                    .camera
                    .world_to_screen((x - width / 2.0, y + height / 2.0));
                let bottom_right = self
                    .camera
                    .world_to_screen((x + width / 2.0, y + height / 2.0));

                // Add the lines for the rectangle
                lines.push((top_left, top_right));
                lines.push((bottom_left, bottom_right));
                lines.push((top_left, bottom_left));
                lines.push((top_right, bottom_right));
            }
        }

        lines
    }
}
