//! Sprite rendering with egui: camera, texture caches and the render queue.
//!
//! Rendering is split in two: [`RenderEngine::render`] turns a scene into a
//! z-sorted queue of draw commands in viewport-relative screen space, and the
//! caller paints them with [`paint_sprite`] using GPU textures from
//! [`RenderEngine::get_egui_texture`].

use crate::logger::LOGGER;
use crate::physics::{ColliderOutline, ColliderShape};
use crate::scene::{Entity, Scene};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use uuid::Uuid;

mod animation;
mod camera;
mod text;

pub use animation::{DEFAULT_ANIMATION_FPS, advance_animations, parse_frames};
pub use camera::Camera;
pub use text::{HudText, TextAlign, paint_texts, parse_color};

/// One sprite draw command in viewport-relative screen space.
#[derive(Debug, Clone)]
pub struct RenderQueueEntry {
    pub entity_id: Uuid,
    pub texture_id: Uuid,
    pub screen_pos: (f32, f32),
    pub screen_size: (f32, f32),
    /// Part of the texture to show, in texture coordinates (0..1). A min
    /// above the max mirrors the sprite along that axis.
    pub uv: egui::Rect,
    /// Color the texture is multiplied by (premultiplied alpha); white
    /// shows the image as is.
    pub tint: egui::Color32,
    /// Rotation around the sprite center, in radians (entity attribute is in degrees).
    pub rotation: f32,
    pub z: f32,
}

/// The whole texture, unmirrored.
const FULL_UV: egui::Rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

/// An entity's `frame_size` attribute (Vector2), when it describes frames of
/// at least one pixel: the entity's image is then a sprite sheet.
pub fn frame_size(entity: &Entity) -> Option<(f32, f32)> {
    entity
        .get_vector2("frame_size")
        .filter(|(w, h)| w.is_finite() && h.is_finite() && *w >= 1.0 && *h >= 1.0)
}

/// Frame number `frame` of a sprite sheet: the image cut into a grid of
/// `frame_size` cells (at most the image's size), numbered left to right,
/// then top to bottom; numbers wrap around. Returns the frame's texture
/// rectangle and its size in pixels.
pub fn sheet_frame(
    image_size: (u32, u32),
    frame_size: (f32, f32),
    frame: i64,
) -> (egui::Rect, (f32, f32)) {
    let (image_w, image_h) = (image_size.0.max(1) as f32, image_size.1.max(1) as f32);
    let (frame_w, frame_h) = (frame_size.0.min(image_w), frame_size.1.min(image_h));
    let columns = ((image_w / frame_w).floor() as i64).max(1);
    let rows = ((image_h / frame_h).floor() as i64).max(1);
    let index = frame.rem_euclid(columns * rows);
    let (column, row) = (index % columns, index / columns);
    let uv = egui::Rect::from_min_size(
        egui::pos2(
            column as f32 * frame_w / image_w,
            row as f32 * frame_h / image_h,
        ),
        egui::vec2(frame_w / image_w, frame_h / image_h),
    );
    (uv, (frame_w, frame_h))
}

/// The part of an entity's image its sprite shows (texture rectangle and
/// size in pixels): the frame selected by its `frame` attribute when it has
/// a `frame_size`, otherwise the whole image.
fn sprite_frame(entity: &Entity, image_size: (u32, u32)) -> (egui::Rect, (f32, f32)) {
    match frame_size(entity) {
        Some(frame_size) => {
            let frame = entity
                .get_float("frame")
                .filter(|frame| frame.is_finite())
                .unwrap_or(0.0)
                .floor() as i64;
            sheet_frame(image_size, frame_size, frame)
        }
        None => (FULL_UV, (image_size.0 as f32, image_size.1 as f32)),
    }
}

/// Paint part of a texture (`uv`, see [`RenderQueueEntry::uv`]) into `rect`,
/// multiplied by `tint` and rotated by `rotation` radians around the rect's
/// center.
pub fn paint_sprite(
    painter: &egui::Painter,
    texture_id: egui::TextureId,
    rect: egui::Rect,
    uv: egui::Rect,
    tint: egui::Color32,
    rotation: f32,
) {
    let center = rect.center();
    let (sin, cos) = rotation.sin_cos();
    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ];
    // Corner by corner, so a mirrored uv rect mirrors the image
    let uvs = [
        egui::pos2(uv.min.x, uv.min.y),
        egui::pos2(uv.max.x, uv.min.y),
        egui::pos2(uv.max.x, uv.max.y),
        egui::pos2(uv.min.x, uv.max.y),
    ];

    let mut mesh = egui::Mesh::with_texture(texture_id);
    for (corner, uv) in corners.iter().zip(uvs) {
        let rel = *corner - center;
        let rotated = egui::vec2(rel.x * cos - rel.y * sin, rel.x * sin + rel.y * cos);
        mesh.vertices.push(egui::epaint::Vertex {
            pos: center + rotated,
            uv,
            color: tint,
        });
    }
    mesh.indices.extend([0, 1, 2, 0, 2, 3]);
    painter.add(mesh);
}

/// Outline color of sensors (trigger zones), whatever their shape.
pub const SENSOR_OUTLINE_COLOR: egui::Color32 = egui::Color32::GREEN;

/// Paint a collider outline given in viewport-relative screen space (see
/// [`RenderEngine::render_colliders`]); `origin` is the viewport's top-left.
/// Solid circles are red and solid rectangles blue; sensors are
/// [`SENSOR_OUTLINE_COLOR`].
pub fn paint_collider_outline(
    painter: &egui::Painter,
    outline: &ColliderOutline,
    origin: egui::Pos2,
) {
    let center = origin + egui::vec2(outline.center.0, outline.center.1);
    let color = |solid: egui::Color32| {
        if outline.is_sensor {
            SENSOR_OUTLINE_COLOR
        } else {
            solid
        }
    };
    match outline.shape {
        ColliderShape::Circle => {
            painter.circle_stroke(
                center,
                outline.size.0 / 2.0,
                egui::Stroke::new(1.0, color(egui::Color32::RED)),
            );
        }
        ColliderShape::Rectangle => {
            let (half_w, half_h) = (outline.size.0 / 2.0, outline.size.1 / 2.0);
            let (sin, cos) = outline.rotation.sin_cos();
            let corners = [
                (-half_w, -half_h),
                (half_w, -half_h),
                (half_w, half_h),
                (-half_w, half_h),
            ]
            .map(|(x, y)| center + egui::vec2(x * cos - y * sin, x * sin + y * cos));
            painter.add(egui::Shape::closed_line(
                corners.to_vec(),
                egui::Stroke::new(1.0, color(egui::Color32::BLUE)),
            ));
        }
    }
}

/// Decoded RGBA pixels of one image file.
#[derive(Debug, Clone)]
pub struct TextureInfo {
    pub data: Vec<u8>,
    /// Width and height in pixels.
    pub dimensions: (u32, u32),
}

/// How a scene's default camera maps onto a viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameView {
    /// Render camera showing the camera entity's frame, scaled to fit.
    pub camera: Camera,
    /// Viewport-relative rect `(x, y, width, height)` covered by the camera
    /// frame; the rest of the viewport is letterboxing.
    pub frame: (f32, f32, f32, f32),
}

/// The frame a scene's default camera shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraFrame {
    /// World-space top-left corner. The camera entity's `x`/`y` is the
    /// frame's center.
    pub world_min: (f32, f32),
    /// World-space size: the camera's `width` × `height` divided by its
    /// `zoom`.
    pub world_size: (f32, f32),
    /// The camera's `width` × `height`: the frame's size in its own pixels,
    /// which is the coordinate space of [`HudText`].
    pub size: (f32, f32),
}

impl CameraFrame {
    /// A world position in the frame's own pixels: (0, 0) is the frame's
    /// top-left corner, and the frame is [`size`](Self::size) pixels big.
    pub fn world_to_pixels(&self, world: (f32, f32)) -> (f32, f32) {
        (
            (world.0 - self.world_min.0) * self.size.0 / self.world_size.0,
            (world.1 - self.world_min.1) * self.size.1 / self.world_size.1,
        )
    }
}

/// The scene's default camera frame, or `None` when the scene has no usable
/// camera (missing, or a size or zoom that isn't positive).
pub fn camera_frame(scene: &Scene) -> Option<CameraFrame> {
    let camera = scene.get_entity(scene.default_camera?).ok()?;
    let zoom = camera.get_camera_zoom();
    let size = (camera.get_camera_width(), camera.get_camera_height());
    let world_size = (size.0 / zoom, size.1 / zoom);
    let positive = |v: f32| v.is_finite() && v > 0.0;
    if ![size.0, size.1, world_size.0, world_size.1]
        .into_iter()
        .all(positive)
    {
        return None;
    }
    Some(CameraFrame {
        world_min: (
            camera.get_x() - world_size.0 / 2.0,
            camera.get_y() - world_size.1 / 2.0,
        ),
        world_size,
        size,
    })
}

/// Fit the scene's default camera frame (see [`camera_frame`]) into a
/// viewport of `viewport_size`, scaled uniformly and letterboxed when the
/// aspect ratios differ. Returns `None` when the scene has no usable camera.
pub fn game_view(scene: &Scene, viewport_size: (f32, f32)) -> Option<GameView> {
    let frame = camera_frame(scene)?;
    let (viewport_w, viewport_h) = viewport_size;
    if !(viewport_w > 0.0 && viewport_h > 0.0) {
        return None;
    }
    let (frame_w, frame_h) = frame.world_size;

    let scale = (viewport_w / frame_w).min(viewport_h / frame_h);
    let shown = (frame_w * scale, frame_h * scale);
    let offset = ((viewport_w - shown.0) / 2.0, (viewport_h - shown.1) / 2.0);

    Some(GameView {
        camera: Camera {
            position: (
                frame.world_min.0 - offset.0 / scale,
                frame.world_min.1 - offset.1 / scale,
            ),
            zoom: scale,
        },
        frame: (offset.0, offset.1, shown.0, shown.1),
    })
}

/// Turns scenes into draw commands and keeps decoded images and GPU textures
/// cached by file path.
#[derive(Clone)]
pub struct RenderEngine {
    viewport_size: (f32, f32),
    texture_cache: HashMap<Uuid, TextureInfo>,
    // GPU-side textures, uploaded once per texture and reused every frame.
    // TextureHandle is an Arc internally, so cloning the engine shares them.
    egui_textures: HashMap<Uuid, egui::TextureHandle>,
    // Images that failed to load; not retried until the cache is cleared
    failed_textures: HashSet<Uuid>,
    // Invalid `tint` values already reported
    reported_tints: HashSet<String>,
    // Cell size of the editor grid, in world units
    grid_size: f32,
    pub camera: Camera,
}

/// Cell size of [`RenderEngine::get_grid_lines`] unless set otherwise.
pub const DEFAULT_GRID_SIZE: f32 = 32.0;

impl Default for RenderEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderEngine {
    pub fn new() -> Self {
        Self {
            viewport_size: (0.0, 0.0),
            texture_cache: HashMap::new(),
            egui_textures: HashMap::new(),
            failed_textures: HashSet::new(),
            reported_tints: HashSet::new(),
            grid_size: DEFAULT_GRID_SIZE,
            camera: Camera::new(),
        }
    }

    /// Set the cell size of [`get_grid_lines`](Self::get_grid_lines), in
    /// world units; sizes below 1 (or not finite) are ignored.
    pub fn set_grid_size(&mut self, size: f32) {
        if size.is_finite() && size >= 1.0 {
            self.grid_size = size;
        }
    }

    pub fn grid_size(&self) -> f32 {
        self.grid_size
    }

    /// Deterministic cache key for an image path.
    pub fn path_to_uuid(path: &Path) -> Uuid {
        Uuid::new_v5(&Uuid::NAMESPACE_URL, path.to_string_lossy().as_bytes())
    }

    /// Decode (once) the image at `path` and return its cache key, or `None`
    /// if it can't be loaded. Failures are logged once per path.
    fn load_texture(&mut self, path: &Path) -> Option<Uuid> {
        let texture_id = Self::path_to_uuid(path);
        if self.texture_cache.contains_key(&texture_id) {
            return Some(texture_id);
        }
        if self.failed_textures.contains(&texture_id) {
            return None;
        }
        match image::open(path) {
            Ok(image) => {
                let rgba = image.to_rgba8();
                self.texture_cache.insert(
                    texture_id,
                    TextureInfo {
                        dimensions: rgba.dimensions(),
                        data: rgba.into_raw(),
                    },
                );
                Some(texture_id)
            }
            Err(e) => {
                LOGGER.warning(format!("Can't load image {}: {}", path.display(), e));
                self.failed_textures.insert(texture_id);
                None
            }
        }
    }

    /// Decoded pixels of an image that has been rendered at least once.
    pub fn get_texture(&self, path: &Path) -> Option<(&Vec<u8>, (u32, u32))> {
        self.texture_cache
            .get(&Self::path_to_uuid(path))
            .map(|info| (&info.data, info.dimensions))
    }

    /// Build the z-sorted (ascending) draw queue for the scene's entities
    /// that have an image and are inside the viewport.
    pub fn render(&mut self, scene: &Scene) -> Vec<RenderQueueEntry> {
        let mut render_queue = Vec::new();

        for (entity_id, entity) in &scene.entities {
            let Ok(image_path) = entity.get_image(0) else {
                continue;
            };
            let Some(texture_id) = self.load_texture(image_path) else {
                continue;
            };
            let Some(texture_info) = self.texture_cache.get(&texture_id) else {
                continue;
            };

            let rotation = entity
                .get_float("rotation")
                .map_or(0.0, |degrees| degrees.to_radians());
            let scale = entity.get_vector2("scale").unwrap_or((1.0, 1.0));

            let (mut uv, pixel_size) = sprite_frame(entity, texture_info.dimensions);
            if entity.get_bool("flip_x") == Some(true) {
                std::mem::swap(&mut uv.min.x, &mut uv.max.x);
            }
            if entity.get_bool("flip_y") == Some(true) {
                std::mem::swap(&mut uv.min.y, &mut uv.max.y);
            }

            let screen_pos = self
                .camera
                .world_to_screen((entity.get_x(), entity.get_y()));
            let width = pixel_size.0 * self.camera.zoom * scale.0;
            let height = pixel_size.1 * self.camera.zoom * scale.1;

            // Viewport culling
            if screen_pos.0 <= self.viewport_size.0
                && screen_pos.0 + width >= 0.0
                && screen_pos.1 <= self.viewport_size.1
                && screen_pos.1 + height >= 0.0
            {
                let tint = self.sprite_tint(entity);
                render_queue.push(RenderQueueEntry {
                    entity_id: *entity_id,
                    texture_id,
                    screen_pos,
                    screen_size: (width, height),
                    uv,
                    tint,
                    rotation,
                    z: entity.get_z(),
                });
            }
        }

        // Lower z values are drawn first; total_cmp keeps the order total
        // (and the sort panic-free) even if a z is NaN
        render_queue.sort_by(|a, b| a.z.total_cmp(&b.z));
        render_queue
    }

    /// The color an entity's sprite is multiplied by: its `tint` attribute
    /// (`#rrggbb` or `#rrggbbaa`, default white) faded by its `opacity`
    /// (0 to 1, default 1). An invalid tint is reported once and ignored.
    fn sprite_tint(&mut self, entity: &Entity) -> egui::Color32 {
        let tint = match entity.get_string("tint") {
            Some(text) => parse_color(text).unwrap_or_else(|e| {
                if self.reported_tints.insert(text.to_string()) {
                    LOGGER.warning(format!("Entity '{}': {}", entity.name, e));
                }
                egui::Color32::WHITE
            }),
            None => egui::Color32::WHITE,
        };
        match entity.get_float("opacity") {
            Some(opacity) if opacity.is_finite() => tint.gamma_multiply(opacity.clamp(0.0, 1.0)),
            _ => tint,
        }
    }

    /// Project world-space collider outlines into viewport space, dropping
    /// the ones that can't be visible.
    pub fn render_colliders(&self, outlines: &[ColliderOutline]) -> Vec<ColliderOutline> {
        let zoom = self.camera.zoom;
        let (viewport_w, viewport_h) = self.viewport_size;
        outlines
            .iter()
            .filter_map(|outline| {
                let center = self.camera.world_to_screen(outline.center);
                let size = (outline.size.0 * zoom, outline.size.1 * zoom);
                // Bounding circle: correct for any rotation
                let radius = size.0.hypot(size.1) / 2.0;
                let visible = center.0 + radius >= 0.0
                    && center.0 - radius <= viewport_w
                    && center.1 + radius >= 0.0
                    && center.1 - radius <= viewport_h;
                visible.then_some(ColliderOutline {
                    center,
                    size,
                    ..*outline
                })
            })
            .collect()
    }

    /// Get (or lazily upload) the GPU texture for a cached texture ID.
    /// The upload happens only once; subsequent calls reuse the handle.
    /// Images larger than the GPU's maximum texture size are downscaled.
    pub fn get_egui_texture(
        &mut self,
        ctx: &egui::Context,
        texture_id: Uuid,
    ) -> Option<egui::TextureHandle> {
        if let Some(handle) = self.egui_textures.get(&texture_id) {
            return Some(handle.clone());
        }

        let info = self.texture_cache.get(&texture_id)?;
        let max_side = ctx.input(|i| i.max_texture_side) as u32;
        let image = color_image_within(info, max_side)?;
        let handle = ctx.load_texture(format!("texture_{}", texture_id), image, Default::default());
        self.egui_textures.insert(texture_id, handle.clone());
        Some(handle)
    }

    pub fn update_viewport_size(&mut self, width: f32, height: f32) {
        self.viewport_size = (width, height);
    }

    pub fn get_viewport_size(&self) -> (f32, f32) {
        self.viewport_size
    }

    /// Clear all caches and reset the camera.
    pub fn cleanup(&mut self) {
        self.clear_cache();
        self.camera.reset();
    }

    /// Forget one image, e.g. after its file changed on disk.
    pub fn unload_texture(&mut self, path: &Path) {
        let texture_id = Self::path_to_uuid(path);
        self.texture_cache.remove(&texture_id);
        self.egui_textures.remove(&texture_id);
        self.failed_textures.remove(&texture_id);
    }

    /// Forget all decoded images and GPU textures; they are reloaded from
    /// disk on the next render.
    pub fn clear_cache(&mut self) {
        self.texture_cache.clear();
        self.egui_textures.clear();
        self.failed_textures.clear();
    }

    /// Bytes of decoded image data held in the cache.
    pub fn get_memory_usage(&self) -> usize {
        self.texture_cache.values().map(|tex| tex.data.len()).sum()
    }

    pub fn get_texture_info(&self, texture_id: &Uuid) -> Option<&TextureInfo> {
        self.texture_cache.get(texture_id)
    }

    /// Grid lines in screen space, aligned to world coordinates so the grid
    /// sticks to the world at any camera position and zoom. Lines are a grid
    /// size apart (see [`set_grid_size`](Self::set_grid_size)), or a power
    /// of two times that while zoomed out, so they never get denser than
    /// ~24 px on screen.
    pub fn get_grid_lines(&self) -> Vec<((f32, f32), (f32, f32))> {
        let mut lines = Vec::new();
        let zoom = self.camera.zoom.max(0.0001);

        // Cells of the grid size, merged while zoomed out until lines are at
        // least 24 px apart
        let mut spacing = self.grid_size;
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

    /// The four edges, in screen space, of the frame the scene's default
    /// camera shows (see [`game_view`]).
    pub fn get_game_camera_bounds(&self, scene: &Scene) -> Vec<((f32, f32), (f32, f32))> {
        let Some(camera) = scene
            .default_camera
            .and_then(|id| scene.get_entity(id).ok())
        else {
            return Vec::new();
        };
        let zoom = camera.get_camera_zoom();
        if zoom <= 0.0 {
            return Vec::new();
        }
        let (x, y) = (camera.get_x(), camera.get_y());
        let half_w = camera.get_camera_width() / zoom / 2.0;
        let half_h = camera.get_camera_height() / zoom / 2.0;

        let top_left = self.camera.world_to_screen((x - half_w, y - half_h));
        let top_right = self.camera.world_to_screen((x + half_w, y - half_h));
        let bottom_left = self.camera.world_to_screen((x - half_w, y + half_h));
        let bottom_right = self.camera.world_to_screen((x + half_w, y + half_h));

        vec![
            (top_left, top_right),
            (bottom_left, bottom_right),
            (top_left, bottom_left),
            (top_right, bottom_right),
        ]
    }
}

/// The texture as an egui image no larger than `max_side` on either side.
fn color_image_within(info: &TextureInfo, max_side: u32) -> Option<egui::ColorImage> {
    let (width, height) = info.dimensions;
    if width <= max_side && height <= max_side {
        return Some(egui::ColorImage::from_rgba_unmultiplied(
            [width as usize, height as usize],
            &info.data,
        ));
    }
    let image = image::RgbaImage::from_raw(width, height, info.data.clone())?;
    let resized = image::DynamicImage::ImageRgba8(image)
        .resize(max_side, max_side, image::imageops::FilterType::Triangle)
        .to_rgba8();
    let size = [resized.width() as usize, resized.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(
        size,
        resized.as_raw(),
    ))
}
