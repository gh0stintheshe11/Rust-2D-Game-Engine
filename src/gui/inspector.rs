use crate::audio_engine::AudioEngine;
use crate::ecs::{AttributeType, AttributeValue, Entity};
use crate::gui::gui_state::{GuiState, SelectedItem};
use crate::gui::scene_hierarchy::utils::format_file_size;
use crate::logger::LOGGER;
use crate::project_manager::ProjectManager;
use eframe::egui;
use eframe::egui::{ColorImage, TextureOptions, Vec2};
use image;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct Inspector {
    // Maps attribute's id to its editing state value
    editing_states: HashMap<Uuid, String>,
    show_metadata_popup: bool,
    metadata_new_name: String,
    metadata_new_type: AttributeType,
    metadata_new_value: AttributeValue,
    metadata_error_message: String,
    data_updated: bool,
    audio_engine: AudioEngine,
    delete_mode: bool,
    // Cached preview data for the currently selected file, so we don't
    // decode images / audio metadata from disk every frame
    preview_image: Option<(PathBuf, egui::TextureHandle, (u32, u32))>,
    preview_audio_duration: Option<(PathBuf, Option<f32>)>,
}

impl Default for Inspector {
    fn default() -> Self {
        Self::new()
    }
}

impl Inspector {
    pub fn new() -> Self {
        Self {
            editing_states: HashMap::new(),
            show_metadata_popup: false,
            metadata_new_name: String::new(),
            metadata_new_type: AttributeType::String,
            metadata_new_value: AttributeValue::String(String::new()),
            metadata_error_message: String::new(),
            data_updated: false,
            audio_engine: AudioEngine::new(),
            delete_mode: false,
            preview_image: None,
            preview_audio_duration: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        match &gui_state.selected_item {
            SelectedItem::Entity(scene_id, entity_id) => {
                let scene_id = *scene_id;
                let entity_id = *entity_id;

                // First show entity details
                self.show_entity_details(ui, ctx, scene_id, entity_id, gui_state);

                // Then get a new borrow for images and sounds
                if let Some(scene_manager) = &gui_state.scene_manager {
                    if let Some(scene) = scene_manager.get_scene(scene_id) {
                        if let Ok(entity) = scene.get_entity(entity_id) {
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
                    }
                }
            }
            SelectedItem::Scene(scene_id) => self.show_scene_details(ui, *scene_id, gui_state),
            SelectedItem::File(file_path) => {
                let file_path = file_path.clone();
                self.show_file_details(ui, &file_path, gui_state);
            }
            SelectedItem::Asset(_scene_id, _entity_id, asset_path) => {
                let asset_path = asset_path.clone();
                self.show_file_details(ui, &asset_path, gui_state);
            }
            SelectedItem::None => {
                ui.label("No item selected.");
            }
        }
    }

    // Display scene information
    fn show_scene_details(&self, ui: &mut egui::Ui, scene_id: Uuid, gui_state: &GuiState) {
        if let Some(scene_manager) = &gui_state.scene_manager {
            if let Some(scene) = scene_manager.get_scene(scene_id) {
                ui.label("Scene Details");
                ui.separator();
                ui.label(format!("Name: {}", scene.name));
                ui.label(format!("ID: {}", scene_id));
                ui.label(format!("Number of Entities: {}", scene.entities.len()));
            } else {
                ui.label("Scene not found.");
            }
        } else {
            ui.label("Scene manager is not initialized.");
        }
    }

    // Display file information
    fn show_file_details(&mut self, ui: &mut egui::Ui, file_path: &Path, gui_state: &mut GuiState) {
        if let Ok(metadata) = fs::metadata(file_path) {
            if metadata.is_file() {
                let extension = file_path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                match extension.as_str() {
                    "png" | "jpg" | "jpeg" | "gif" => {
                        // Decode + upload the preview only when the selected file changes
                        let cached =
                            matches!(&self.preview_image, Some((p, _, _)) if p == file_path);
                        if !cached {
                            self.preview_image = image::open(file_path).ok().map(|img| {
                                let width = img.width();
                                let height = img.height();
                                let rgba_img = img.to_rgba8();
                                let image = ColorImage::from_rgba_unmultiplied(
                                    [width as usize, height as usize],
                                    rgba_img.as_raw(),
                                );
                                let texture = ui.ctx().load_texture(
                                    format!("preview_{}", file_path.display()),
                                    image,
                                    TextureOptions::default(),
                                );
                                (file_path.to_path_buf(), texture, (width, height))
                            });
                        }

                        if let Some((_, texture, (width, height))) = &self.preview_image {
                            let width = *width;
                            let height = *height;
                            let aspect_ratio = width as f32 / height as f32;

                            // Calculate display size with maximum dimensions
                            let available_width = ui.available_width();
                            let max_preview_width = available_width.min(400.0); // Maximum width of 400 pixels
                            let max_preview_height = 300.0; // Maximum height of 300 pixels

                            let mut display_width;
                            let mut display_height;

                            if aspect_ratio > 1.0 {
                                // Wide image
                                display_width = max_preview_width;
                                display_height = display_width / aspect_ratio;
                                if display_height > max_preview_height {
                                    display_height = max_preview_height;
                                    display_width = display_height * aspect_ratio;
                                }
                            } else {
                                // Tall image
                                display_height = max_preview_height;
                                display_width = display_height * aspect_ratio;
                                if display_width > max_preview_width {
                                    display_width = max_preview_width;
                                    display_height = display_width / aspect_ratio;
                                }
                            }

                            // Center the image
                            let available_width = ui.available_width();
                            let padding = ((available_width - display_width) / 2.0).max(0.0);
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.add_space(padding);
                                egui::Frame::NONE
                                    .stroke(egui::Stroke::new(1.0_f32, egui::Color32::GRAY))
                                    .show(ui, |ui| {
                                        ui.image((
                                            texture.id(),
                                            Vec2::new(display_width, display_height),
                                        ));
                                    });
                            });
                            ui.add_space(8.0);

                            // Show file info
                            ui.separator();
                            ui.label("Path:");
                            ui.label(format!("{}", file_path.to_string_lossy()));
                            ui.separator();
                            ui.label(format!("Size: {}", format_file_size(metadata.len())));
                            ui.separator();
                            ui.label(format!("Dimensions: {} x {} pixels", width, height));
                        }
                    }
                    "mp3" | "wav" | "ogg" => {
                        ui.horizontal(|ui| {
                            if ui.button("▶ Play").clicked() {
                                if let Err(e) = self.audio_engine.play_sound_immediate(file_path) {
                                    println!("Failed to play sound: {}", e);
                                }
                            }

                            if ui.button("⏹ Stop").clicked() {
                                self.audio_engine.stop_immediate();
                            }
                        });

                        ui.separator();
                        ui.label("Path:");
                        ui.label(format!("{}", file_path.to_string_lossy()));
                        ui.separator();

                        // Probe the file for its duration only when the selection changes
                        let cached =
                            matches!(&self.preview_audio_duration, Some((p, _)) if p == file_path);
                        if !cached {
                            let duration = match self.audio_engine.get_audio_duration(file_path) {
                                Ok(d) => Some(d),
                                Err(e) => {
                                    println!("Failed to get audio duration: {}", e);
                                    None
                                }
                            };
                            self.preview_audio_duration = Some((file_path.to_path_buf(), duration));
                        }

                        if let Some((_, Some(duration))) = &self.preview_audio_duration {
                            let duration = *duration;
                            let duration_text = {
                                let total_seconds = duration.round() as i64;
                                let seconds = total_seconds % 60;
                                let minutes = (total_seconds / 60) % 60;
                                let hours = (total_seconds / 3600) % 24;
                                let days = total_seconds / 86400;

                                if days > 0 {
                                    format!(
                                        "Duration: {}d {}h {}m {}s",
                                        days, hours, minutes, seconds
                                    )
                                } else if hours > 0 {
                                    format!("Duration: {}h {}m {}s", hours, minutes, seconds)
                                } else if minutes > 0 {
                                    format!("Duration: {}m {}s", minutes, seconds)
                                } else {
                                    format!("Duration: {}s", seconds)
                                }
                            };
                            ui.label(duration_text);
                            ui.separator();
                        }

                        ui.label(format!("Size: {}", format_file_size(metadata.len())));
                    }
                    "lua" | "rs" => {
                        if ui.button("Edit Script").clicked() {
                            gui_state.open_script_request = Some(file_path.to_path_buf());
                        }
                        ui.separator();
                        ui.label("Path:");
                        ui.label(format!("{}", file_path.to_string_lossy()));
                        ui.separator();
                        ui.label(format!("Size: {}", format_file_size(metadata.len())));
                    }
                    "ttf" | "otf" => {
                        ui.separator();
                        ui.label("Path:");
                        ui.label(format!("{}", file_path.to_string_lossy()));
                        ui.separator();
                        ui.label(format!("Size: {}", format_file_size(metadata.len())));
                    }
                    _ => {
                        ui.label("Unsupported file type.");
                        ui.separator();
                        ui.label("Path:");
                        ui.label(format!("{}", file_path.to_string_lossy()));
                        ui.separator();
                        ui.label(format!("Size: {}", format_file_size(metadata.len())));
                    }
                }
            } else {
                ui.label("Not a file.");
            }
        } else {
            ui.label("Failed to retrieve file metadata.");
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
        if let Some(scene_manager) = &mut gui_state.scene_manager {
            if let Some(scene) = scene_manager.get_scene_mut(scene_id) {
                if let Ok(entity) = scene.get_entity_mut(entity_id) {
                    ui.label(entity.name.to_string());
                    ui.separator();
                    ui.label(format!("ID: {}", entity_id));
                    ui.separator();
                    ui.label(format!("Scene ID: {}", scene_id));
                    ui.separator();

                    for (&attribute_id, attribute) in &entity.attributes.clone() {
                        self.display_attribute(
                            ui,
                            attribute_id,
                            &attribute.name,
                            &attribute.value,
                            entity,
                        );
                    }

                    // Buttons in same row with even spacing
                    ui.horizontal(|ui| {
                        let available_width = ui.available_width();
                        let button_width = 20.0; // 10.0 is spacing between buttons

                        ui.add_space((available_width - 2.0 * button_width - 4.0) / 2.0);
                        if ui
                            .add_sized([button_width, 20.0], egui::Button::new("➕"))
                            .clicked()
                        {
                            self.show_metadata_popup = true;
                            self.metadata_new_name.clear();
                            self.metadata_new_type = AttributeType::String;
                            self.metadata_new_value = AttributeValue::String(String::new());
                            self.metadata_error_message.clear();
                        }

                        ui.add_space(4.0);

                        if ui
                            .add_sized(
                                [button_width, 20.0],
                                egui::Button::new(if self.delete_mode { "⭕" } else { "➖" }),
                            )
                            .clicked()
                        {
                            self.delete_mode = !self.delete_mode;
                        }
                    });

                    if self.show_metadata_popup {
                        self.show_metadata_popup(ctx, ui, entity);
                    }
                } else {
                    ui.label("Entity not found.");
                }
            } else {
                ui.label("Scene not found.");
            }
        } else {
            ui.label("Scene manager is not initialized.");
        }

        // Save project if any updates
        if self.data_updated {
            self.data_updated = false;
            if let Some(scene_manager) = &gui_state.scene_manager {
                if let Err(err) = ProjectManager::save_project_full(
                    &gui_state.project_path,
                    gui_state.project_metadata.as_ref().unwrap(),
                    scene_manager,
                ) {
                    println!(
                        "Error saving project after modifying/adding an attribute: {}",
                        err
                    );
                } else {
                    println!("Saved project after modifying/adding an attribute.");
                }
            }
        }
    }

    /// Add metadata popup, type must be in Entity's attribute types
    // TODO: handle Vector2
    fn show_metadata_popup(
        &mut self,
        ctx: &egui::Context,
        _ui: &mut egui::Ui,
        entity: &mut Entity,
    ) {
        egui::Window::new("Add Attribute")
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.label("Enter Attribute Name:");
                ui.text_edit_singleline(&mut self.metadata_new_name);

                ui.label("Select Attribute Type:");
                egui::ComboBox::from_label("Type")
                    .selected_text(format!("{:?}", self.metadata_new_value))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(
                                &mut self.metadata_new_value,
                                AttributeValue::Integer(0),
                                "Integer",
                            )
                            .clicked()
                        {
                            self.metadata_new_type = AttributeType::Integer;
                        }
                        if ui
                            .selectable_value(
                                &mut self.metadata_new_value,
                                AttributeValue::Float(0.0),
                                "Float",
                            )
                            .clicked()
                        {
                            self.metadata_new_type = AttributeType::Float;
                        }
                        if ui
                            .selectable_value(
                                &mut self.metadata_new_value,
                                AttributeValue::String(String::new()),
                                "String",
                            )
                            .clicked()
                        {
                            self.metadata_new_type = AttributeType::String;
                        }
                        if ui
                            .selectable_value(
                                &mut self.metadata_new_value,
                                AttributeValue::Boolean(false),
                                "Boolean",
                            )
                            .clicked()
                        {
                            self.metadata_new_type = AttributeType::Boolean;
                        }
                    });

                if !self.metadata_error_message.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.metadata_error_message);
                }

                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        if self.is_valid_identifier(&self.metadata_new_name) {
                            let full_name = format!("metadata_{}", self.metadata_new_name);
                            let value = self.metadata_new_value.clone();
                            let attr_type = self.metadata_new_type.clone();

                            match entity.create_attribute(&full_name, attr_type, value) {
                                Ok(attribute_id) => {
                                    println!("Created {} with ID: {}", full_name, attribute_id);
                                    self.data_updated = true;
                                    self.show_metadata_popup = false;
                                    self.metadata_new_name.clear();
                                    self.metadata_new_type = AttributeType::String;
                                    self.metadata_new_value = AttributeValue::String(String::new());
                                    self.metadata_error_message.clear();
                                }
                                Err(error) => {
                                    println!("Failed to create {}: {}", full_name, error);
                                    self.metadata_error_message = error;
                                }
                            }
                        } else {
                            self.metadata_error_message =
                                "Invalid identifier name. Use alphanumeric or underscores."
                                    .to_string();
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        self.show_metadata_popup = false;
                        self.metadata_new_name.clear();
                        self.metadata_new_type = AttributeType::String;
                        self.metadata_new_value = AttributeValue::String(String::new());
                        self.metadata_error_message.clear();
                    }
                });
            });
    }

    /// Display individual attribute with editing
    fn display_attribute(
        &mut self,
        ui: &mut egui::Ui,
        attribute_id: Uuid,
        attribute_name: &str,
        attribute_value: &AttributeValue,
        entity: &mut Entity,
    ) {
        let temp_value = self
            .editing_states
            .entry(attribute_id)
            .or_insert_with(|| attribute_value.to_string())
            .clone();

        ui.horizontal(|ui| {
            ui.add_space(4.0);
            // Only show delete button in delete mode
            if self.delete_mode && ui.small_button("❌").clicked() {
                if let Err(e) = entity.delete_attribute(attribute_id) {
                    LOGGER.error(format!("Failed to delete attribute: {}", e));
                }
                self.editing_states.remove(&attribute_id);
                self.data_updated = true;
                return;
            }

            let input_width = 80.0; // Fixed width for input
            let total_spacing = 16.0; // Left and right margins
            let available_width = ui.available_width() - input_width - total_spacing;

            // Get the text layout for the full name
            let font = egui::TextStyle::Body.resolve(ui.style());
            let text_layout = ui.fonts_mut(|f| {
                f.layout_no_wrap(
                    attribute_name.to_string(),
                    font.clone(),
                    egui::Color32::WHITE,
                )
            });

            // Left side with label
            ui.scope(|ui| {
                ui.set_width(available_width);
                let display_name = if text_layout.rect.width() > available_width {
                    let mut fit_chars = attribute_name.len();
                    for (i, _) in attribute_name.char_indices() {
                        let test_text = format!("{}...", &attribute_name[..i]);
                        let test_layout = ui.fonts_mut(|f| {
                            f.layout_no_wrap(test_text, font.clone(), egui::Color32::WHITE)
                        });
                        if test_layout.rect.width() > available_width {
                            fit_chars = i.saturating_sub(1);
                            break;
                        }
                    }
                    format!("{}...", &attribute_name[..fit_chars])
                } else {
                    attribute_name.to_string()
                };

                ui.label(egui::RichText::new(display_name).strong())
                    .on_hover_text(attribute_name);
            });

            // Right side with fixed-width input
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                ui.set_width(input_width);

                match attribute_value {
                    AttributeValue::Boolean(_) => {
                        let mut value = temp_value.parse::<bool>().unwrap_or(false);
                        if ui.checkbox(&mut value, "").changed() {
                            if let Err(e) = entity.modify_attribute(
                                attribute_id,
                                None,
                                None,
                                Some(AttributeValue::Boolean(value)),
                            ) {
                                LOGGER.error(format!("Failed to modify attribute: {}", e));
                            }
                            self.editing_states.insert(attribute_id, value.to_string());
                            self.data_updated = true;
                        }
                    }
                    _ => {
                        let response = ui.add(
                            egui::TextEdit::singleline(
                                self.editing_states.get_mut(&attribute_id).unwrap(),
                            )
                            .desired_width(input_width),
                        );

                        if response.lost_focus() {
                            if let Some(new_value) =
                                self.parse_attribute_value(&temp_value, attribute_value)
                            {
                                if let Err(e) = entity.modify_attribute(
                                    attribute_id,
                                    None,
                                    None,
                                    Some(new_value),
                                ) {
                                    LOGGER.error(format!("Failed to modify attribute: {}", e));
                                }
                                self.editing_states.remove(&attribute_id);
                                self.data_updated = true;
                            } else {
                                self.editing_states.remove(&attribute_id);
                            }
                        }
                    }
                }
            });
        });
        ui.separator();
    }

    /// Validate for new attribute name
    fn is_valid_identifier(&self, name: &str) -> bool {
        !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
    }

    /// Parse input value
    fn parse_attribute_value(
        &self,
        input: &str,
        attribute_value: &AttributeValue,
    ) -> Option<AttributeValue> {
        match attribute_value {
            AttributeValue::Integer(_) => input.parse::<i32>().ok().map(AttributeValue::Integer),
            AttributeValue::Float(_) => input.parse::<f32>().ok().map(AttributeValue::Float),
            AttributeValue::String(_) => Some(AttributeValue::String(input.to_string())),
            AttributeValue::Boolean(_) => input.parse::<bool>().ok().map(AttributeValue::Boolean),
            AttributeValue::Vector2(_, _) => {
                let parts: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
                if parts.len() == 2 {
                    if let (Ok(x), Ok(y)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                        return Some(AttributeValue::Vector2(x, y));
                    }
                }
                None
            }
        }
    }
}
