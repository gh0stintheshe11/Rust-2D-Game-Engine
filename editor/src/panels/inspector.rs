use crate::project::AssetType;
use crate::state::{GuiState, SelectedItem, TileBrush};
use eframe::egui;
use rust_2d_game_engine::audio::AudioEngine;
use rust_2d_game_engine::logger::LOGGER;
use rust_2d_game_engine::render::{ONE_WAY_OUTLINE_COLOR, parse_frames, sheet_frame};
use rust_2d_game_engine::scene::{
    AttributeType, AttributeValue, Entity, TileAnimation, TileCollision, Tilemap, attribute_presets,
};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Longest side of the GPU texture used for image previews. Previews are
/// shown at most 400 px wide, and full-size uploads of huge images exceed the
/// GPU's texture size limit.
const PREVIEW_TEXTURE_SIZE: u32 = 512;

/// The attribute field currently being typed in.
struct FieldEdit {
    scene_id: Uuid,
    entity_id: Uuid,
    attribute_id: Uuid,
    text: String,
}

/// The "Add Attribute" popup, bound to the entity it was opened for.
struct NewAttribute {
    scene_id: Uuid,
    entity_id: Uuid,
    name: String,
    value: AttributeValue,
    error: String,
}

/// A decoded image preview, or `None` if the file couldn't be decoded.
type ImagePreview = Option<(egui::TextureHandle, (u32, u32))>;

/// Color of the mark on animated tiles in the palette.
const ANIMATED_TILE_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 170, 60);

/// Speed of a tile animation made in the inspector, in frames per second.
const DEFAULT_TILE_ANIMATION_FPS: f32 = 4.0;

/// The frames typed for a tile's animation, until they're applied.
struct TileAnimationText {
    tile: u32,
    text: String,
    error: Option<String>,
}

/// Most tiles the palette shows.
const MAX_PALETTE_TILES: u32 = 512;

/// Side of a tile button in the palette, in points.
const PALETTE_TILE_SIZE: f32 = 32.0;

pub struct Inspector {
    editing: Option<FieldEdit>,
    new_attribute: Option<NewAttribute>,
    delete_mode: bool,
    /// The entity the panel showed last frame
    shown_entity: Option<(Uuid, Uuid)>,
    /// Set when an edit changed the scene; committed at the end of the frame
    data_updated: bool,
    audio_engine: AudioEngine,
    // Previews for the selected file, so images / audio metadata aren't
    // decoded from disk every frame (failures are cached too)
    preview_image: Option<(PathBuf, ImagePreview)>,
    preview_audio_duration: Option<(PathBuf, Option<f32>)>,
    // The selected tilemap's tileset, for the palette
    tileset_preview: Option<(PathBuf, ImagePreview)>,
    // The brush tile's animation frames being typed, and why they were refused
    tile_animation_text: Option<TileAnimationText>,
    // Snippet queued for the script editor (clicking an attribute name)
    pending_script_insert: Option<String>,
}

impl Default for Inspector {
    fn default() -> Self {
        Self::new()
    }
}

impl Inspector {
    pub fn new() -> Self {
        Self {
            editing: None,
            new_attribute: None,
            delete_mode: false,
            shown_entity: None,
            data_updated: false,
            audio_engine: AudioEngine::new(),
            preview_image: None,
            preview_audio_duration: None,
            tileset_preview: None,
            tile_animation_text: None,
            pending_script_insert: None,
        }
    }

    /// Forget everything tied to the open project (e.g. when another one
    /// is opened).
    pub fn reset(&mut self) {
        self.audio_engine.cleanup();
        self.editing = None;
        self.new_attribute = None;
        self.delete_mode = false;
        self.shown_entity = None;
        self.data_updated = false;
        self.preview_image = None;
        self.preview_audio_duration = None;
        self.tileset_preview = None;
        self.tile_animation_text = None;
        self.pending_script_insert = None;
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        let selected_entity = match gui_state.selected_item {
            SelectedItem::Entity(scene_id, entity_id) => Some((scene_id, entity_id)),
            _ => None,
        };
        if selected_entity != self.shown_entity {
            // Keep what was typed into the previous entity's field
            self.commit_pending_edit(gui_state);
            self.new_attribute = None;
            self.delete_mode = false;
            self.shown_entity = selected_entity;
        }

        match &gui_state.selected_item {
            SelectedItem::Entity(scene_id, entity_id) => {
                let (scene_id, entity_id) = (*scene_id, *entity_id);
                self.show_entity_details(ui, ctx, scene_id, entity_id, gui_state);
            }
            SelectedItem::Scene(scene_id) => self.show_scene_details(ui, *scene_id, gui_state),
            SelectedItem::File(file_path) => {
                let file_path = file_path.clone();
                self.show_file_details(ui, &file_path, gui_state);
            }
            SelectedItem::None => {
                ui.label("No item selected.");
            }
        }

        if self.data_updated {
            self.data_updated = false;
            gui_state.commit_change();
        }

        // Forward any queued snippet to the script editor
        if let Some(snippet) = self.pending_script_insert.take() {
            gui_state.script_insert_request = Some(snippet);
        }
    }

    /// Apply the text being typed into an attribute field, if it parses and
    /// differs from the current value.
    fn commit_pending_edit(&mut self, gui_state: &mut GuiState) {
        let Some(edit) = self.editing.take() else {
            return;
        };
        let entity = gui_state
            .scene_manager
            .as_mut()
            .and_then(|manager| manager.get_scene_mut(edit.scene_id))
            .and_then(|scene| scene.get_entity_mut(edit.entity_id).ok());
        if let Some(entity) = entity
            && apply_text(entity, edit.attribute_id, &edit.text)
        {
            gui_state.commit_change();
        }
    }

    // Display scene information
    fn show_scene_details(&self, ui: &mut egui::Ui, scene_id: Uuid, gui_state: &GuiState) {
        let Some(scene) = gui_state
            .scene_manager
            .as_ref()
            .and_then(|manager| manager.get_scene(scene_id))
        else {
            ui.label("Scene not found.");
            return;
        };
        ui.label("Scene Details");
        ui.separator();
        ui.label(format!("Name: {}", scene.name));
        ui.label(format!("ID: {}", scene_id));
        ui.label(format!("Number of Entities: {}", scene.entities.len()));
    }

    // Display file information
    fn show_file_details(&mut self, ui: &mut egui::Ui, file_path: &Path, gui_state: &mut GuiState) {
        let Ok(metadata) = fs::metadata(file_path) else {
            ui.label("Failed to retrieve file metadata.");
            return;
        };
        if !metadata.is_file() {
            ui.label("Not a file.");
            return;
        }
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "png" | "jpg" | "jpeg" | "gif" => self.show_image_preview(ui, file_path),
            "mp3" | "wav" | "ogg" | "flac" => self.show_audio_preview(ui, file_path),
            "lua" if ui.button("Edit Script").clicked() => {
                gui_state.open_script_request = Some(file_path.to_path_buf());
            }
            _ => {}
        }

        ui.separator();
        ui.label("Path:");
        ui.label(file_path.to_string_lossy());
        ui.separator();
        ui.label(format!("Size: {}", format_file_size(metadata.len())));
    }

    fn show_image_preview(&mut self, ui: &mut egui::Ui, file_path: &Path) {
        // Decode + upload only when the selected file changes
        if !matches!(&self.preview_image, Some((p, _)) if p == file_path) {
            let preview = match image::open(file_path) {
                Ok(image) => {
                    let size = (image.width(), image.height());
                    let thumbnail = image
                        .thumbnail(PREVIEW_TEXTURE_SIZE, PREVIEW_TEXTURE_SIZE)
                        .to_rgba8();
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        [thumbnail.width() as usize, thumbnail.height() as usize],
                        thumbnail.as_raw(),
                    );
                    let texture = ui.ctx().load_texture(
                        format!("preview_{}", file_path.display()),
                        color_image,
                        egui::TextureOptions::default(),
                    );
                    Some((texture, size))
                }
                Err(e) => {
                    LOGGER.warning(format!("Can't preview {}: {}", file_path.display(), e));
                    None
                }
            };
            self.preview_image = Some((file_path.to_path_buf(), preview));
        }

        let Some((_, Some((texture, (width, height))))) = &self.preview_image else {
            ui.label("This image can't be decoded.");
            return;
        };

        // Fit into 400 x 300, keeping the aspect ratio
        let scale = (ui.available_width().min(400.0) / *width as f32)
            .min(300.0 / *height as f32)
            .min(1.0);
        let display = egui::vec2(*width as f32 * scale, *height as f32 * scale);

        ui.add_space(8.0);
        ui.vertical_centered(|ui| {
            egui::Frame::NONE
                .stroke(egui::Stroke::new(1.0_f32, egui::Color32::GRAY))
                .show(ui, |ui| {
                    ui.image((texture.id(), display));
                });
        });
        ui.add_space(8.0);
        ui.separator();
        ui.label(format!("Dimensions: {} x {} pixels", width, height));
    }

    fn show_audio_preview(&mut self, ui: &mut egui::Ui, file_path: &Path) {
        ui.horizontal(|ui| {
            if ui.button("▶ Play").clicked() {
                // Re-read the file every time: it may have changed, and
                // previews shouldn't accumulate in memory
                self.audio_engine.clear_cache();
                if let Err(e) = self.audio_engine.play_sound_immediate(file_path) {
                    LOGGER.warning(format!("Can't play {}: {}", file_path.display(), e));
                }
            }
            if ui.button("⏹ Stop").clicked() {
                self.audio_engine.stop_immediate();
            }
        });

        // Probe the file for its duration only when the selection changes
        if !matches!(&self.preview_audio_duration, Some((p, _)) if p == file_path) {
            let duration = self.audio_engine.get_audio_duration(file_path).ok();
            self.preview_audio_duration = Some((file_path.to_path_buf(), duration));
        }
        if let Some((_, Some(duration))) = &self.preview_audio_duration {
            ui.separator();
            ui.label(format!("Duration: {}", format_duration(*duration)));
        }
    }

    /// Display entity information
    fn show_entity_details(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        scene_id: Uuid,
        entity_id: Uuid,
        gui_state: &mut GuiState,
    ) {
        let Some(entity) = gui_state
            .scene_manager
            .as_mut()
            .and_then(|manager| manager.get_scene_mut(scene_id))
            .and_then(|scene| scene.get_entity_mut(entity_id).ok())
        else {
            ui.label("Entity not found.");
            return;
        };

        ui.label(entity.name.to_string());
        ui.separator();
        ui.label(format!("ID: {}", entity_id));
        ui.separator();

        let attributes: Vec<(Uuid, String, AttributeValue)> = entity
            .attributes
            .iter()
            .map(|(id, attribute)| (*id, attribute.name.clone(), attribute.value.clone()))
            .collect();
        for (attribute_id, name, value) in attributes {
            self.display_attribute(
                ui,
                (scene_id, entity_id),
                attribute_id,
                &name,
                &value,
                entity,
            );
        }

        ui.horizontal(|ui| {
            let button_width = 20.0;
            ui.add_space((ui.available_width() - 2.0 * button_width - 4.0) / 2.0);
            if ui
                .add_sized([button_width, 20.0], egui::Button::new("➕"))
                .on_hover_text("Add an attribute")
                .clicked()
            {
                self.new_attribute = Some(NewAttribute {
                    scene_id,
                    entity_id,
                    name: String::new(),
                    value: AttributeValue::Float(0.0),
                    error: String::new(),
                });
            }
            ui.add_space(4.0);
            if ui
                .add_sized(
                    [button_width, 20.0],
                    egui::Button::new(if self.delete_mode { "⭕" } else { "➖" }),
                )
                .on_hover_text("Delete attributes")
                .clicked()
            {
                self.delete_mode = !self.delete_mode;
            }
        });
        ui.vertical_centered(|ui| {
            ui.menu_button("Add Feature", |ui| {
                if Self::show_presets_menu(ui, entity) {
                    self.data_updated = true;
                }
            })
            .response
            .on_hover_text(
                "Add the attributes a feature needs (a trigger zone, an animation, …), \
                 with working values",
            );
        });

        if self
            .new_attribute
            .as_ref()
            .is_some_and(|popup| (popup.scene_id, popup.entity_id) == (scene_id, entity_id))
        {
            self.show_new_attribute_popup(ctx, entity);
        }

        if entity.tilemap.is_some() {
            ui.separator();
            self.show_tilemap_section(
                ui,
                ctx,
                entity,
                &gui_state.project_path,
                &mut gui_state.tile_brush,
                &mut gui_state.painting_tiles,
            );
        }

        ui.separator();
        ui.label("Images:");
        for path in &entity.images {
            ui.label(path.to_string_lossy());
        }
        ui.separator();
        ui.label("Sounds:");
        for path in &entity.sounds {
            ui.label(path.to_string_lossy());
        }
    }

    /// An entity's tilemap: its tileset and tile size, and the paint mode and
    /// palette that paint it in the viewport.
    fn show_tilemap_section(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        entity: &mut Entity,
        project_path: &Path,
        brush: &mut TileBrush,
        painting: &mut bool,
    ) {
        let Some(tilemap) = entity.tilemap.as_mut() else {
            return;
        };
        ui.strong("Tilemap");

        let current = tilemap
            .tileset
            .file_name()
            .map_or("Choose an image…".to_string(), |name| {
                name.to_string_lossy().into_owned()
            });
        egui::ComboBox::from_label("Tileset")
            .selected_text(current)
            .show_ui(ui, |ui| {
                // Listed only while the list is open
                for image in project_images(project_path) {
                    let name = image
                        .strip_prefix(project_path.join("assets").join("images"))
                        .unwrap_or(&image)
                        .to_string_lossy()
                        .into_owned();
                    if ui
                        .selectable_label(tilemap.tileset == image, name)
                        .clicked()
                    {
                        tilemap.tileset = image;
                        self.data_updated = true;
                    }
                }
            });

        ui.horizontal(|ui| {
            ui.label("Tile size");
            let mut size = tilemap.tile_size;
            let width = ui.add(
                egui::DragValue::new(&mut size.0)
                    .range(1..=1024)
                    .suffix(" px"),
            );
            ui.label("×");
            let height = ui.add(
                egui::DragValue::new(&mut size.1)
                    .range(1..=1024)
                    .suffix(" px"),
            );
            tilemap.tile_size = size;
            // One undo step per change, not one per frame of a drag
            let done = |response: &egui::Response| {
                response.drag_stopped() || (response.changed() && !response.dragged())
            };
            if done(&width) || done(&height) {
                self.data_updated = true;
            }
        });
        ui.label(format!("{} tiles", tilemap.tile_count()));

        // Tiles painted without a tileset would be invisible, yet solid
        let has_tileset = !tilemap.tileset.as_os_str().is_empty();
        ui.add_enabled_ui(has_tileset, |ui| {
            ui.horizontal(|ui| {
                ui.toggle_value(painting, "🖌 Paint")
                    .on_hover_text(
                        "Left-drag in the viewport to paint with the selected tile; Esc stops",
                    )
                    .on_disabled_hover_text("Choose a tileset first");
                if ui
                    .selectable_label(*brush == TileBrush::Eraser, "Eraser")
                    .clicked()
                {
                    *brush = TileBrush::Eraser;
                }
            });
        });
        self.show_tile_palette(ui, ctx, tilemap, brush);
        self.show_tile_collision(ui, tilemap, *brush);
        self.show_tile_animation(ui, tilemap, *brush);
    }

    /// The brush tile's animation: frames typed like a sprite animation's
    /// ("5,9" or "5-8"), applied with Enter or when the field loses focus,
    /// and their speed.
    fn show_tile_animation(&mut self, ui: &mut egui::Ui, tilemap: &mut Tilemap, brush: TileBrush) {
        let TileBrush::Tile(tile) = brush else {
            return;
        };
        if tilemap.tileset.as_os_str().is_empty() {
            return;
        }
        let animation = tilemap.tile_animation(tile).cloned();
        let shown = |animation: &Option<TileAnimation>| {
            animation.as_ref().map_or(String::new(), |animation| {
                let frames: Vec<String> = animation.frames().iter().map(u32::to_string).collect();
                frames.join(",")
            })
        };
        // A new brush tile shows its own frames
        if self
            .tile_animation_text
            .as_ref()
            .is_none_or(|typed| typed.tile != tile)
        {
            self.tile_animation_text = Some(TileAnimationText {
                tile,
                text: shown(&animation),
                error: None,
            });
        }
        let Some(typed) = self.tile_animation_text.as_mut() else {
            return;
        };
        let mut apply = false;
        ui.horizontal(|ui| {
            ui.label("Animation");
            let response = ui.add(
                egui::TextEdit::singleline(&mut typed.text)
                    .hint_text("frames, e.g. 5,9")
                    .desired_width(110.0),
            );
            apply = response.lost_focus();
            if let Some(animation) = &animation {
                let mut fps = animation.fps();
                let speed = ui.add(
                    egui::DragValue::new(&mut fps)
                        .range(0.5..=60.0)
                        .speed(0.1)
                        .suffix(" fps"),
                );
                if speed.changed()
                    && let Ok(faster) = TileAnimation::new(animation.frames().to_vec(), fps)
                {
                    tilemap.set_tile_animation(tile, Some(faster));
                }
                // One undo step per change, not one per frame of a drag
                if speed.drag_stopped() || (speed.changed() && !speed.dragged()) {
                    self.data_updated = true;
                }
            }
        });
        if apply {
            let text = typed.text.trim();
            let result = if text.is_empty() {
                Ok(None)
            } else {
                let fps = animation
                    .as_ref()
                    .map_or(DEFAULT_TILE_ANIMATION_FPS, TileAnimation::fps);
                parse_frames(text).and_then(|frames| TileAnimation::new(frames, fps).map(Some))
            };
            match result {
                Ok(new) if new != animation => {
                    typed.text = shown(&new);
                    typed.error = None;
                    tilemap.set_tile_animation(tile, new);
                    self.data_updated = true;
                }
                Ok(_) => typed.error = None,
                Err(e) => typed.error = Some(e),
            }
        }
        if let Some(error) = &typed.error {
            ui.colored_label(egui::Color32::RED, error);
        }
    }

    /// How the brush's tile collides, as a choice.
    fn show_tile_collision(&mut self, ui: &mut egui::Ui, tilemap: &mut Tilemap, brush: TileBrush) {
        let TileBrush::Tile(tile) = brush else {
            return;
        };
        if tilemap.tileset.as_os_str().is_empty() {
            return;
        }
        let current = tilemap.tile_collision(tile);
        ui.horizontal(|ui| {
            ui.label(format!("Tile {} is", tile));
            egui::ComboBox::from_id_salt("tile_collision")
                .selected_text(collision_label(current))
                .show_ui(ui, |ui| {
                    for kind in [
                        TileCollision::Solid,
                        TileCollision::OneWay,
                        TileCollision::Passable,
                    ] {
                        let help = match kind {
                            TileCollision::Solid => "Solid from every side",
                            TileCollision::OneWay => {
                                "Solid only from above: bodies jump up through it and land on it"
                            }
                            TileCollision::Passable => "Not solid: decoration",
                        };
                        if ui
                            .selectable_label(current == kind, collision_label(kind))
                            .on_hover_text(help)
                            .clicked()
                            && kind != current
                        {
                            tilemap.set_tile_collision(tile, kind);
                            self.data_updated = true;
                        }
                    }
                });
        });
    }

    /// The tileset's tiles as buttons; clicking one makes it the brush.
    fn show_tile_palette(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        tilemap: &Tilemap,
        brush: &mut TileBrush,
    ) {
        if tilemap.tileset.as_os_str().is_empty() {
            ui.label("Choose a tileset to paint with.");
            return;
        }
        let Some((texture, image_size)) = self.tileset_texture(ctx, &tilemap.tileset) else {
            ui.colored_label(egui::Color32::RED, "The tileset can't be loaded.");
            return;
        };
        let tile = (tilemap.tile_size.0 as f32, tilemap.tile_size.1 as f32);
        let columns = (image_size.0 / tilemap.tile_size.0).max(1);
        let rows = (image_size.1 / tilemap.tile_size.1).max(1);
        let count = columns.saturating_mul(rows);
        // The brush is one of this tileset's tiles (it may come from a
        // larger one)
        if matches!(*brush, TileBrush::Tile(number) if number >= count) {
            *brush = TileBrush::Tile(0);
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
            for number in 0..count.min(MAX_PALETTE_TILES) {
                let (uv, _) = sheet_frame(image_size, tile, number as i64);
                let image =
                    egui::Image::new((texture.id(), egui::Vec2::splat(PALETTE_TILE_SIZE))).uv(uv);
                let selected = *brush == TileBrush::Tile(number);
                let collision = tilemap.tile_collision(number);
                let response = ui
                    .add(egui::Button::image(image).selected(selected))
                    .on_hover_text(format!("Tile {} · {}", number, collision_label(collision)));
                if response.clicked() {
                    *brush = TileBrush::Tile(number);
                }
                // Animated tiles are marked with a small triangle ("play")
                if tilemap.tile_animation(number).is_some() {
                    let corner = response.rect.shrink(4.0).right_bottom();
                    ui.painter().add(egui::Shape::convex_polygon(
                        vec![
                            corner + egui::vec2(-7.0, -8.0),
                            corner + egui::vec2(0.0, -4.0),
                            corner + egui::vec2(-7.0, 0.0),
                        ],
                        ANIMATED_TILE_COLOR,
                        egui::Stroke::NONE,
                    ));
                }
                // Tiles that aren't solid are marked: one-way ones with
                // their top edge, as in collider outlines
                let rect = response.rect.shrink(3.0);
                match collision {
                    TileCollision::Solid => {}
                    TileCollision::OneWay => {
                        ui.painter().line_segment(
                            [rect.left_top(), rect.right_top()],
                            egui::Stroke::new(2.0, ONE_WAY_OUTLINE_COLOR),
                        );
                    }
                    TileCollision::Passable => {
                        let corners = [
                            rect.left_top(),
                            rect.right_top(),
                            rect.right_bottom(),
                            rect.left_bottom(),
                            rect.left_top(),
                        ];
                        ui.painter().extend(egui::Shape::dashed_line(
                            &corners,
                            egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY),
                            3.0,
                            3.0,
                        ));
                    }
                }
            }
        });
        if count > MAX_PALETTE_TILES {
            ui.label(format!(
                "The first {} of {} tiles",
                MAX_PALETTE_TILES, count
            ));
        }
    }

    /// The tileset as a texture, and its size in pixels; cached per path.
    fn tileset_texture(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
    ) -> Option<(egui::TextureHandle, (u32, u32))> {
        if self
            .tileset_preview
            .as_ref()
            .map(|(cached, _)| cached.as_path())
            != Some(path)
        {
            let max_side = ctx.input(|i| i.max_texture_side) as u32;
            let loaded = image::open(path).ok().map(|image| {
                let size = (image.width(), image.height());
                // The texture may be smaller than the image; tile positions
                // are fractions, so the palette still finds them
                let image = if size.0 > max_side || size.1 > max_side {
                    image.resize(max_side, max_side, image::imageops::FilterType::Triangle)
                } else {
                    image
                };
                let rgba = image.to_rgba8();
                let pixels = egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_raw(),
                );
                let texture =
                    ctx.load_texture("tileset_palette", pixels, egui::TextureOptions::NEAREST);
                (texture, size)
            });
            self.tileset_preview = Some((path.to_path_buf(), loaded));
        }
        self.tileset_preview.as_ref()?.1.clone()
    }

    /// The presets menu: one submenu per group; choosing a preset adds the
    /// attributes it needs that the entity lacks. Returns whether any were
    /// added.
    fn show_presets_menu(ui: &mut egui::Ui, entity: &mut Entity) -> bool {
        let presets = attribute_presets();
        let mut groups: Vec<&str> = presets.iter().map(|preset| preset.group).collect();
        groups.dedup();
        let mut added = 0;
        for group in groups {
            ui.menu_button(group, |ui| {
                for preset in presets.iter().filter(|preset| preset.group == group) {
                    let names: Vec<&str> =
                        preset.attributes.iter().map(|(name, _)| *name).collect();
                    let response = ui
                        .add_enabled(!entity.has_preset(preset), egui::Button::new(preset.name))
                        .on_hover_text(format!(
                            "{}\n\nAdds {}",
                            preset.description,
                            names.join(", ")
                        ))
                        .on_disabled_hover_text("The entity has these attributes already");
                    if response.clicked() {
                        added = entity.apply_preset(preset);
                        LOGGER.info(format!(
                            "Added {} to '{}' ({} attributes)",
                            preset.name, entity.name, added
                        ));
                        ui.close();
                    }
                }
            });
        }
        added > 0
    }

    /// The "Add Attribute" popup.
    fn show_new_attribute_popup(&mut self, ctx: &egui::Context, entity: &mut Entity) {
        let Some(popup) = &mut self.new_attribute else {
            return;
        };
        let mut close = false;
        egui::Window::new("Add Attribute")
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.label(format!("New attribute for '{}':", entity.name));
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut popup.name);
                });

                egui::ComboBox::from_label("Type")
                    .selected_text(format!("{:?}", popup.value.data_type()))
                    .show_ui(ui, |ui| {
                        for (label, value) in [
                            ("Float", AttributeValue::Float(0.0)),
                            ("Integer", AttributeValue::Integer(0)),
                            ("Boolean", AttributeValue::Boolean(false)),
                            ("String", AttributeValue::String(String::new())),
                            ("Vector2", AttributeValue::Vector2(0.0, 0.0)),
                        ] {
                            let selected = popup.value.data_type() == value.data_type();
                            if ui.selectable_label(selected, label).clicked() {
                                popup.value = value;
                            }
                        }
                    });

                // Initial value editor for Vector2 (other types start from
                // their defaults and are edited afterwards in the inspector)
                if let AttributeValue::Vector2(x, y) = &mut popup.value {
                    ui.horizontal(|ui| {
                        ui.label("x:");
                        ui.add(egui::DragValue::new(x).speed(1.0));
                        ui.label("y:");
                        ui.add(egui::DragValue::new(y).speed(1.0));
                    });
                }

                if !popup.error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &popup.error);
                }

                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        if !is_valid_identifier(&popup.name) {
                            popup.error = "Use letters, digits and underscores, \
                                           not starting with a digit."
                                .to_string();
                        } else {
                            let data_type: AttributeType = popup.value.data_type();
                            match entity.create_attribute(
                                &popup.name,
                                data_type,
                                popup.value.clone(),
                            ) {
                                Ok(_) => {
                                    self.data_updated = true;
                                    close = true;
                                }
                                Err(error) => popup.error = error,
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
        if close {
            self.new_attribute = None;
        }
    }

    /// One attribute row: its name (click to insert `get_attribute` into
    /// the script editor) and an editor for its value.
    fn display_attribute(
        &mut self,
        ui: &mut egui::Ui,
        (scene_id, entity_id): (Uuid, Uuid),
        attribute_id: Uuid,
        name: &str,
        value: &AttributeValue,
        entity: &mut Entity,
    ) {
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            if self.delete_mode && ui.small_button("❌").clicked() {
                match entity.delete_attribute(attribute_id) {
                    Ok(true) => self.data_updated = true,
                    Ok(false) => {}
                    Err(e) => LOGGER.error(format!("Failed to delete attribute: {}", e)),
                }
                return;
            }

            let input_width = 80.0;
            let label_width = (ui.available_width() - input_width - 16.0).max(20.0);
            ui.scope(|ui| {
                ui.set_width(label_width);
                // `truncate` elides with "…" at a character boundary
                let label = egui::Label::new(egui::RichText::new(name).strong())
                    .truncate()
                    .sense(egui::Sense::click());
                if ui
                    .add(label)
                    .on_hover_text(format!(
                        "{}\n\nClick to insert get_attribute(\"{}\") into the script editor",
                        name, name
                    ))
                    .clicked()
                {
                    self.pending_script_insert =
                        Some(format!("get_attribute(scene_id, entity_id, \"{}\")", name));
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                ui.set_width(input_width);
                if let AttributeValue::Boolean(current) = value {
                    let mut checked = *current;
                    if ui.checkbox(&mut checked, "").changed() {
                        match entity.modify_attribute(
                            attribute_id,
                            None,
                            None,
                            Some(AttributeValue::Boolean(checked)),
                        ) {
                            Ok(()) => self.data_updated = true,
                            Err(e) => LOGGER.error(format!("Failed to modify attribute: {}", e)),
                        }
                    }
                    return;
                }

                // Only the field being typed in keeps its own text; all
                // others always show the current value
                let is_editing = self
                    .editing
                    .as_ref()
                    .is_some_and(|edit| edit.attribute_id == attribute_id);
                let mut text = match &self.editing {
                    Some(edit) if is_editing => edit.text.clone(),
                    _ => value.to_string(),
                };
                let response = ui.add(
                    egui::TextEdit::singleline(&mut text)
                        .id(ui.make_persistent_id(("attribute", entity_id, attribute_id)))
                        .desired_width(input_width),
                );
                if response.has_focus() {
                    self.editing = Some(FieldEdit {
                        scene_id,
                        entity_id,
                        attribute_id,
                        text,
                    });
                } else if response.lost_focus() && is_editing {
                    self.editing = None;
                    if apply_text(entity, attribute_id, &text) {
                        self.data_updated = true;
                    }
                }
            });
        });
        ui.separator();
    }
}

/// Parse `text` as the attribute's current type and store it. Returns
/// whether the value changed; invalid input is reported and ignored.
/// How a tile collides, in words.
fn collision_label(collision: TileCollision) -> &'static str {
    match collision {
        TileCollision::Solid => "Solid",
        TileCollision::OneWay => "One-way platform",
        TileCollision::Passable => "Passable",
    }
}

/// The project's images (`assets/images` and its folders), sorted.
fn project_images(project_path: &Path) -> Vec<PathBuf> {
    let extensions = AssetType::Image.valid_extensions();
    let mut images = Vec::new();
    let mut folders = vec![project_path.join("assets").join("images")];
    while let Some(folder) = folders.pop() {
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                folders.push(path);
            } else if path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    extensions.contains(&extension.to_ascii_lowercase().as_str())
                })
            {
                images.push(path);
            }
        }
    }
    images.sort();
    images
}

fn apply_text(entity: &mut Entity, attribute_id: Uuid, text: &str) -> bool {
    let Ok(attribute) = entity.get_attribute(attribute_id) else {
        return false;
    };
    let Some(new_value) = parse_attribute_value(text, &attribute.value) else {
        LOGGER.warning(format!(
            "'{}' is not a valid {:?} for '{}'",
            text,
            attribute.value.data_type(),
            attribute.name
        ));
        return false;
    };
    if new_value == attribute.value {
        return false;
    }
    match entity.modify_attribute(attribute_id, None, None, Some(new_value)) {
        Ok(()) => true,
        Err(e) => {
            LOGGER.error(format!("Failed to modify attribute: {}", e));
            false
        }
    }
}

/// Attribute names must work as plain identifiers in scripts.
fn is_valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Parse user input as a value of the same type as `current`.
fn parse_attribute_value(input: &str, current: &AttributeValue) -> Option<AttributeValue> {
    let input = input.trim();
    match current {
        AttributeValue::Integer(_) => input.parse::<i32>().ok().map(AttributeValue::Integer),
        AttributeValue::Float(_) => input.parse::<f32>().ok().map(AttributeValue::Float),
        AttributeValue::String(_) => Some(AttributeValue::String(input.to_string())),
        AttributeValue::Boolean(_) => input.parse::<bool>().ok().map(AttributeValue::Boolean),
        AttributeValue::Vector2(_, _) => {
            let (x, y) = input.split_once(',')?;
            Some(AttributeValue::Vector2(
                x.trim().parse().ok()?,
                y.trim().parse().ok()?,
            ))
        }
    }
}

fn format_duration(seconds: f32) -> String {
    let total = seconds.round() as u64;
    let (hours, minutes, seconds) = (total / 3600, (total / 60) % 60, total % 60);
    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

fn format_file_size(size_in_bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size_in_bytes >= GB {
        format!("{:.2} GB", size_in_bytes as f64 / GB as f64)
    } else if size_in_bytes >= MB {
        format!("{:.2} MB", size_in_bytes as f64 / MB as f64)
    } else if size_in_bytes >= KB {
        format!("{:.2} KB", size_in_bytes as f64 / KB as f64)
    } else {
        format!("{} B", size_in_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_attribute_value() {
        assert_eq!(
            parse_attribute_value(" 3 , -4.5 ", &AttributeValue::Vector2(0.0, 0.0)),
            Some(AttributeValue::Vector2(3.0, -4.5))
        );
        assert_eq!(
            parse_attribute_value("x", &AttributeValue::Integer(0)),
            None
        );
        assert_eq!(
            parse_attribute_value("2.5", &AttributeValue::Float(0.0)),
            Some(AttributeValue::Float(2.5))
        );
    }

    #[test]
    fn test_identifiers() {
        assert!(is_valid_identifier("jump_velocity"));
        assert!(is_valid_identifier("_x2"));
        assert!(!is_valid_identifier("2x"));
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("速度"));
    }

    #[test]
    fn test_apply_text_only_reports_real_changes() {
        let mut entity = Entity::new(Uuid::new_v4(), "e").unwrap();
        let x = entity.get_attribute_by_name("x").unwrap().id;
        assert!(!apply_text(&mut entity, x, "0"), "unchanged value");
        assert!(apply_text(&mut entity, x, "12"));
        assert_eq!(entity.get_x(), 12.0);
        assert!(!apply_text(&mut entity, x, "inf"), "rejected by validation");
        assert!(!apply_text(&mut entity, x, "abc"), "not a number");
        assert_eq!(entity.get_x(), 12.0);
    }
}
