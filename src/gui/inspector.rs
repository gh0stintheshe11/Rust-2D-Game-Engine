use eframe::egui;
use crate::gui::gui_state::{GuiState, SelectedItem};
use crate::ecs::{AttributeValue, AttributeType, Attribute, Entity};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;
use crate::project_manager::ProjectManager;

pub struct Inspector {
    // Maps attribute's id to its editing state value
    editing_states: HashMap<Uuid, String>,
    show_metadata_popup: bool,
    metadata_new_name: String,
    metadata_new_type: AttributeType,
    metadata_new_value: AttributeValue,
    metadata_error_message: String,
    data_updated: bool,
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
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        match &gui_state.selected_item {
            SelectedItem::Scene(scene_id) => self.show_scene_details(ui, *scene_id, gui_state),
            SelectedItem::Entity(scene_id, entity_id) => {
                self.show_entity_details(ui, ctx, *scene_id, *entity_id, gui_state)
            }
            SelectedItem::File(file_path) => self.show_file_details(ui, file_path),
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
                ui.label(format!("Number of Resources: {}", scene.resources.len()));
            } else {
                ui.label("Scene not found.");
            }
        } else {
            ui.label("Scene manager is not initialized.");
        }
    }

    // Display file information
    fn show_file_details(&self, ui: &mut egui::Ui, file_path: &Path) {
        ui.label("File Details");
        ui.separator();
        ui.label(format!("Path: {}", file_path.display()));

        if let Ok(metadata) = std::fs::metadata(file_path) {
            if metadata.is_file() {
                ui.label(format!("Size: {} bytes", metadata.len()));
            } else {
                ui.label("Not a file.");
            }
        } else {
            ui.label("Failed to retrieve file metadata.");
        }
    }

    // Display entity information
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
                if let Some(entity) = scene.get_entity_mut(entity_id) {
                    ui.label("Entity Details");
                    ui.separator();
                    // ui.label(format!("Name: {}", entity.name));
                    // ui.label(format!("ID: {}", entity_id));
                    // ui.label(format!("Scene ID: {}", scene_id));

                    // Group attributes by prefix
                    let mut grouped_attributes: HashMap<String, Vec<(Uuid, String, AttributeValue)>> = HashMap::new();
                    for (&attribute_id, attribute) in &entity.attributes {
                        if let Some((prefix, name)) = attribute.name.split_once('_') {
                            grouped_attributes
                                .entry(prefix.to_string())
                                .or_default()
                                .push((attribute_id, name.to_string(), attribute.value.clone()));
                        } else {
                            // If cant find prefix (e.g., no "_" in name), group them as "Other"
                            grouped_attributes
                                .entry("Other".to_string())
                                .or_default()
                                .push((attribute_id, attribute.name.clone(), attribute.value.clone()));
                        }
                    }

                    // Sorting groups, predefined groups first, metadata last
                    let mut sorted_groups: Vec<(String, Vec<(Uuid, String, AttributeValue)>)> =
                        grouped_attributes.into_iter().collect();

                    // Show metadata group at the last
                    sorted_groups.sort_by(|(a, _), (b, _)| {
                        if a == "metadata" {
                            std::cmp::Ordering::Greater
                        } else if b == "metadata" {
                            std::cmp::Ordering::Less
                        } else {
                            a.cmp(b)
                        }
                    });

                    // Display groups and attributes
                    for (prefix, attributes) in sorted_groups {
                        let header_id = ui.make_persistent_id(format!("{}_header", prefix));
                        let header_title = prefix[0..1].to_uppercase() + &prefix[1..];

                        egui::collapsing_header::CollapsingState::load_with_default_open(ctx, header_id, true)
                            .show_header(ui, |ui| {
                                ui.label(header_title);
                            })
                            .body(|ui| {
                                for (attribute_id, name, value) in attributes {
                                    self.display_attribute(ui, attribute_id, &name, &value, entity);
                                }
                            });
                    }

                    // Add Metadata Button
                    if ui.button("Add Metadata").clicked() {
                        self.show_metadata_popup = true;
                        self.metadata_new_name.clear();
                        self.metadata_new_type = AttributeType::String;
                        self.metadata_new_value = AttributeValue::String(String::new());
                        self.metadata_error_message.clear();
                    }

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
                    Path::new(&gui_state.project_path),
                    gui_state.project_metadata.as_ref().unwrap(),
                    scene_manager,
                ) {
                    println!("Error saving project after modifying/adding an attribute: {}", err);
                } else {
                    println!("Saved project after modifying/adding an attribute.");
                }
            }
        }
    }


    /// Add metadata popup, type must be in Entity's attribute types
    // TODO: handle Vector2
    fn show_metadata_popup(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, entity: &mut Entity) {
        egui::Window::new("Add Metadata")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Enter Metadata Name:");
                ui.text_edit_singleline(&mut self.metadata_new_name);

                ui.label("Select Metadata Type:");
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
                                attribute_id => {
                                    println!("Created {} with ID: {}", full_name, attribute_id);
                                    self.data_updated = true;
                                }
                                _ => {
                                    println!("Failed to create {}", full_name);
                                }
                            }

                            self.show_metadata_popup = false;
                            self.metadata_new_name.clear();
                            self.metadata_new_type = AttributeType::String;
                            self.metadata_new_value = AttributeValue::String(String::new());
                            self.metadata_error_message.clear();
                        } else {
                            self.metadata_error_message = "Invalid identifier name. Use alphanumeric or underscores.".to_string();
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
            ui.label(attribute_name);

            let response = ui.text_edit_singleline(self.editing_states.get_mut(&attribute_id).unwrap());

            // Check and save value if the input field lost focus
            if response.lost_focus() {
                if let Some(new_value) = self.parse_attribute_value(&temp_value, attribute_value) {
                    entity.modify_attribute(attribute_id, None, None, Some(new_value));
                    self.editing_states.remove(&attribute_id);
                    self.data_updated = true;
                } else {
                    self.editing_states.remove(&attribute_id);
                }
            }
        });
    }

    /// Validate for new attribute name
    fn is_valid_identifier(&self, name: &str) -> bool {
        !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
    }

    /// Parse input value
    fn parse_attribute_value(&self, input: &str, attribute_value: &AttributeValue) -> Option<AttributeValue> {
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
