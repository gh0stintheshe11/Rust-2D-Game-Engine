use eframe::egui;
use ecs::{AttributeType, AttributeValue};
use crate::gui::gui_state::{GuiState, SelectedItem};
use uuid::Uuid;
use crate::project_manager::ProjectManager;
use std::path::Path;
use crate::ecs;
use crate::ecs::{ResourceType};
use std::collections::HashMap;

const PREDEFINED_ENTITIES: &[(
    &str,
    &[(&str, AttributeType, AttributeValue)],
)] = &[
    (
        "Camera",
        &[
            ("transform_position_x", AttributeType::Float, AttributeValue::Float(0.0)),
            ("transform_position_y", AttributeType::Float, AttributeValue::Float(0.0)),
            ("transform_position_z", AttributeType::Float, AttributeValue::Float(0.0)),
            ("transform_rotation_x", AttributeType::Float, AttributeValue::Float(0.0)),
            ("transform_rotation_y", AttributeType::Float, AttributeValue::Float(0.0)),
            ("transform_rotation_z", AttributeType::Float, AttributeValue::Float(0.0)),
            ("transform_scale_x", AttributeType::Float, AttributeValue::Float(1.0)),
            ("transform_scale_y", AttributeType::Float, AttributeValue::Float(1.0)),
            ("transform_scale_z", AttributeType::Float, AttributeValue::Float(1.0)),
        ],
    ),
];


pub struct SceneHierarchy {
    search_query: String,
    show_create_popup: bool,
    create_item_type: String,
    create_item_name: String,
    selected_item: Option<(String, Uuid)>, // Track selected item (type, ID) (type: Scene, Entity)
    error_message: String,
    // for rename
    renaming_scene: Option<Uuid>,
    renaming_entity: Option<(Uuid, Uuid)>,
    rename_input: String,
    // for manage resource of entity
    manage_resources_entity: Option<(Uuid, Uuid)>, // Track selected entity for manage resource (scene_id, entity_id)
    selected_resources: HashMap<Uuid, bool>,
    show_manage_resource_popup: bool,
    manage_resource_popup_error_message: String,
}

impl SceneHierarchy {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            show_create_popup: false,
            create_item_type: String::new(),
            create_item_name: String::new(),
            selected_item: None,
            error_message: String::new(),
            // for rename
            renaming_scene: None,
            renaming_entity: None,
            rename_input: String::new(),
            // for manage resource of entity
            manage_resources_entity: None,
            selected_resources: HashMap::new(),
            show_manage_resource_popup: false,
            manage_resource_popup_error_message: String::new(),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        // Menu bar
        ui.horizontal(|ui| {
            if ui.button("+").clicked() {
                self.show_create_popup = true;
            }

            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.search_query);
        });

        ui.separator();

        if self.show_create_popup {
            self.render_create_popup(ctx, ui, gui_state);
        } else {
            // Set a default selection
            if self.create_item_type.is_empty() {
                self.create_item_type = "Scene".to_string();
            }
        }

        if self.show_manage_resource_popup {
            if let Some((scene_id, entity_id)) = self.manage_resources_entity {
                self.render_manage_resource_group(ctx, ui, gui_state, scene_id, entity_id);
            }
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut scene_to_delete: Option<Uuid> = None;
                let mut entity_to_delete: Option<(Uuid, Uuid)> = None;
                let mut resource_to_detach: Option<(Uuid, Uuid, Uuid)> = None; // scene_id, entity_id, resource_id

                if let Some(scene_manager) = &gui_state.scene_manager {
                    for (scene_id, scene) in &scene_manager.scenes {
                        let id = ui.make_persistent_id(scene_id);

                        // Scene is collapsable
                        egui::collapsing_header::CollapsingState::load_with_default_open(ctx, id, true)
                            .show_header(ui, |ui| {
                                let selected = self
                                    .selected_item
                                    .as_ref()
                                    .map_or(false, |(item_type, id)| item_type == "Scene" && id == scene_id);

                                let response = ui.selectable_label(selected, &scene.name);
                                if response.clicked() {
                                    self.selected_item = Some(("Scene".to_string(), *scene_id));
                                    gui_state.selected_item = SelectedItem::Scene(*scene_id);
                                }

                                // Handle right-click context menu for scene
                                response.context_menu(|ui| {
                                    if ui.button("Rename").clicked() {
                                        self.renaming_scene = Some(*scene_id);
                                        self.rename_input = scene.name.clone();
                                        ui.close_menu();
                                    }
                                    if ui.button("Delete").clicked() {
                                        scene_to_delete = Some(*scene_id);
                                        ui.close_menu();
                                    }
                                });
                            })
                            .body(|ui| {
                                for (entity_id, entity) in &scene.entities {
                                    // Apply search filter to entities
                                    if !self.search_query.is_empty()
                                        && !entity.name.to_lowercase().contains(&self.search_query.to_lowercase())
                                    {
                                        continue;
                                    }

                                    // If entity has resources, show as collapse header, otherwise show as label
                                    if !entity.resource_list.is_empty() {
                                        // Entity is collapsible
                                        let entity_ui_id = ui.make_persistent_id(entity_id);

                                        egui::collapsing_header::CollapsingState::load_with_default_open(ctx, entity_ui_id, true)
                                            .show_header(ui, |ui| {
                                                let selected = self
                                                    .selected_item
                                                    .as_ref()
                                                    .map_or(false, |(item_type, id)| item_type == "Entity" && id == entity_id);

                                                let response = ui.selectable_label(selected, format!("📌 {}", entity.name));
                                                if response.clicked() {
                                                    self.selected_item = Some(("Entity".to_string(), *entity_id));
                                                    gui_state.selected_item = SelectedItem::Entity(*scene_id, *entity_id);
                                                }

                                                // Handle right-click context menu for entity
                                                response.context_menu(|ui| {
                                                    if ui.button("Manage Resources").clicked() {
                                                        self.manage_resources_entity = Some((*scene_id, *entity_id));
                                                        self.show_manage_resource_popup = true;
                                                        ui.close_menu();
                                                    }
                                                    if ui.button("Rename").clicked() {
                                                        self.renaming_entity = Some((*scene_id, *entity_id));
                                                        self.rename_input = entity.name.clone();
                                                        ui.close_menu();
                                                    }
                                                    if ui.button("Delete").clicked() {
                                                        entity_to_delete = Some((*scene_id, *entity_id));
                                                        ui.close_menu();
                                                    }
                                                });
                                            }).body(|ui| {

                                            // Display resources in entity
                                            for resource_id in &entity.resource_list {
                                                if let Some(resource) = scene.resources.get(resource_id) {
                                                    let resource_selected = self
                                                        .selected_item
                                                        .as_ref()
                                                        .map_or(false, |(item_type, id)| item_type == "Resource" && id == resource_id);

                                                    let response = ui.selectable_label(resource_selected, format!("📄 {}", resource.name));

                                                    if response.clicked() {
                                                        // Update selected item state to Resource
                                                        self.selected_item = Some(("Resource".to_string(), *resource_id));
                                                        gui_state.selected_item = SelectedItem::Resource(*scene_id, *resource_id);
                                                    }

                                                    // Handle right-click context menu for resource
                                                    response.context_menu(|ui| {
                                                        if ui.button("Detach").clicked() {
                                                            resource_to_detach = Some((*scene_id, *entity_id, *resource_id));
                                                            ui.close_menu();
                                                        }
                                                    });
                                                } else {
                                                    println!("Resource {resource_id} not found in entity {entity_id}.");
                                                }
                                            }
                                        });
                                    } else {
                                        ui.horizontal(|ui| {
                                            let selected = self
                                                .selected_item
                                                .as_ref()
                                                .map_or(false, |(item_type, id)| item_type == "Entity" && id == entity_id);

                                            let response = ui.selectable_label(selected, format!("📌 {}", entity.name));
                                            if response.clicked() {
                                                self.selected_item = Some(("Entity".to_string(), *entity_id));
                                                gui_state.selected_item = SelectedItem::Entity(*scene_id, *entity_id);
                                            }

                                            // Handle right-click context menu for entity
                                            response.context_menu(|ui| {
                                                if ui.button("Manage Resources").clicked() {
                                                    self.manage_resources_entity = Some((*scene_id, *entity_id));
                                                    self.show_manage_resource_popup = true;
                                                    ui.close_menu();
                                                }
                                                if ui.button("Rename").clicked() {
                                                    self.renaming_entity = Some((*scene_id, *entity_id));
                                                    self.rename_input = entity.name.clone();
                                                    ui.close_menu();
                                                }
                                                if ui.button("Delete").clicked() {
                                                    entity_to_delete = Some((*scene_id, *entity_id));
                                                    ui.close_menu();
                                                }
                                            });
                                        });
                                    }
                                }
                            });
                    }
                } else {
                    ui.label("No scenes loaded.");
                }

                // Handle renaming after the UI loop to avoid borrow issues
                if let Some(scene_id) = self.renaming_scene {
                    self.open_rename_popup(ctx, ui, "Rename Scene", scene_id, None, gui_state);
                }

                if let Some((scene_id, entity_id)) = self.renaming_entity {
                    self.open_rename_popup(ctx, ui, "Rename Entity", scene_id, Some(entity_id), gui_state);
                }

                // Handle deletion after the UI loop, avoid Rust borrow issues
                if let Some(scene_id) = scene_to_delete {
                    self.delete_scene(scene_id, gui_state);
                }

                if let Some((scene_id, entity_id)) = entity_to_delete {
                    self.delete_entity(scene_id, entity_id, gui_state);
                }

                if let Some((scene_id, entity_id, resource_id)) = resource_to_detach {
                    self.detach_resource(scene_id, entity_id, resource_id, gui_state);
                }
            });

    }

    fn render_create_popup(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        egui::Window::new("Create New Node")
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.label("Select item to create:");

                ui.vertical(|ui| {
                    let available_width = ui.available_width();

                    if ui
                        .add_sized([available_width, 24.0], egui::SelectableLabel::new(self.create_item_type == "Scene", "Scene"))
                        .clicked()
                    {
                        self.create_item_type = "Scene".to_string();
                    }

                    if ui
                        .add_sized([available_width, 24.0], egui::SelectableLabel::new(self.create_item_type == "Entity", "Entity"))
                        .clicked()
                    {
                        self.create_item_type = "Entity".to_string();
                    }

                    // Predefined Entity
                    for (name, _) in PREDEFINED_ENTITIES.iter() {
                        if ui
                            .add_sized([available_width, 24.0], egui::SelectableLabel::new(self.create_item_type == name.to_string(), name.to_string()))
                            .clicked()
                        {
                            self.create_item_type = name.to_string();
                        }
                    }

                    if ui
                        .add_sized([available_width, 24.0], egui::SelectableLabel::new(self.create_item_type == "Resource", "Resource"))
                        .clicked()
                    {
                        self.create_item_type = "Resource".to_string();
                    }
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.create_item_name);
                });
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        if !self.create_item_name.trim().is_empty() {
                            if let Some(scene_manager) = gui_state.scene_manager.as_mut() {
                                match self.create_item_type.as_str() {
                                    "Scene" => {
                                        self.create_new_scene(gui_state);
                                        if self.error_message.is_empty() {
                                            self.show_create_popup = false;
                                        }
                                    }
                                    "Entity" => {
                                        self.create_new_entity("Entity".to_string(), gui_state);
                                        if self.error_message.is_empty() {
                                            self.show_create_popup = false;
                                        }
                                    }
                                    "Resource" => {
                                        self.create_new_resource(gui_state);
                                        if self.error_message.is_empty() {
                                            self.show_create_popup = false;
                                        }
                                    }
                                    name => {
                                        // Predefined entity creation
                                        if PREDEFINED_ENTITIES.iter().any(|(predefined_name, _)| predefined_name.to_string() == name.to_string()) {
                                            self.create_new_entity(name.to_string(), gui_state);
                                            if self.error_message.is_empty() {
                                                self.show_create_popup = false;
                                            }
                                        } else {
                                            self.error_message = "Invalid item type selected.".to_string();
                                        }
                                    }
                                    _ => self.error_message = "Invalid item type selected.".to_string(),
                                }
                            } else {
                                self.error_message = "Scene manager is not initialized.".to_string();
                            }
                        } else {
                            self.error_message = "Please select an item type and enter a valid name.".to_string();
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        self.show_create_popup = false;
                        self.create_item_name.clear();
                        self.error_message.clear();
                    }
                });

                if !self.error_message.is_empty() {
                    ui.add_space(10.0);
                    ui.colored_label(egui::Color32::RED, &self.error_message);
                }

            });
    }

    fn render_manage_resource_group(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState, scene_id: Uuid, entity_id: Uuid) {

        let mut resource_updated = false;

        egui::Window::new("Manage Resources")
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                if let Some(scene_manager) = &mut gui_state.scene_manager {
                    if let Some(scene) = scene_manager.get_scene_mut(scene_id) {
                        if let Some(entity) = scene.get_entity(entity_id) {

                            let max_height = ctx.available_rect().height() * 0.6;

                            // Get resource list and sort by name
                            let mut resources: Vec<(Uuid, String)> = scene
                                .list_resource()
                                .iter()
                                .map(|(id, name)| (*id, name.to_string()))
                                .collect();
                            resources.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));

                            egui::ScrollArea::vertical()
                                .max_height(max_height)
                                .auto_shrink([false, true])
                                .show(ui, |ui| {
                                    // Display resource list with checkboxes
                                    for (resource_id, resource_name) in &resources {
                                        let is_selected = self
                                            .selected_resources
                                            .entry(*resource_id)
                                            .or_insert(entity.resource_list.contains(&resource_id));
                                        ui.horizontal(|ui| {
                                            ui.checkbox(is_selected, resource_name.to_string());
                                        });
                                    }
                                });

                            ui.separator();

                            ui.horizontal(|ui| {
                                if ui.button("Save").clicked() {
                                    resource_updated = true;
                                    self.show_manage_resource_popup = false;
                                }
                                if ui.button("Cancel").clicked() {
                                    self.show_manage_resource_popup = false;
                                    self.selected_resources.clear();
                                }
                            });
                        } else {
                            self.manage_resource_popup_error_message = "Entity not found.".to_string();
                        }
                    } else {
                        self.manage_resource_popup_error_message = "Scene not found.".to_string();
                    }
                } else {
                    self.manage_resource_popup_error_message = "Scene manager is not initialized.".to_string();
                }

                // Display error message
                if !self.manage_resource_popup_error_message.is_empty() {
                    ui.add_space(10.0);
                    ui.colored_label(egui::Color32::RED, &self.manage_resource_popup_error_message);
                }

            });


        if resource_updated {

            if let Some(scene_manager) = &mut gui_state.scene_manager {
                if let Some(scene) = scene_manager.get_scene_mut(scene_id) {
                    if let Some(entity) = scene.get_entity_mut(entity_id) {

                        // Update the entity's resource list based on selection
                        entity.resource_list.clear();
                        for (resource_id, selected) in &self.selected_resources {
                            if *selected {
                                entity.resource_list.push(*resource_id);
                            }
                        }

                        // Save the project after updating
                        if let Err(err) = ProjectManager::save_project_full(
                            Path::new(&gui_state.project_path),
                            gui_state.project_metadata.as_ref().unwrap(),
                            scene_manager,
                        ) {
                            self.manage_resource_popup_error_message = "Error saving project after managing resources: {err}".to_string();
                        } else {
                            self.manage_resources_entity = None;
                            self.selected_resources.clear();
                            self.manage_resource_popup_error_message.clear();
                            self.show_manage_resource_popup = false;
                        }


                        return;
                    }
                }
            }
            self.manage_resources_entity = None;
            println!("Error saving project after managing resources.");
        }
    }

    /// Create a new scene
    fn create_new_scene(&mut self, gui_state: &mut GuiState) {
        if let Some(scene_manager) = gui_state.scene_manager.as_mut() {
            let name = self.create_item_name.trim().to_string();
            if !name.is_empty() {
                let new_scene_id = scene_manager.create_scene(&name);
                self.selected_item = Some(("Scene".to_string(), new_scene_id));
                gui_state.selected_item = SelectedItem::Scene(new_scene_id);
                println!("Created new scene with ID: {:?}", new_scene_id);

                // Save project after creating the scene
                if let Err(err) = ProjectManager::save_project_full(
                    Path::new(&gui_state.project_path),
                    gui_state.project_metadata.as_ref().unwrap(),
                    scene_manager,
                ) {
                    self.error_message = format!("Error saving project after creating a scene: {}", err);
                } else {
                    self.error_message.clear();
                }
            } else {
                self.error_message = "Scene name cannot be empty.".to_string();
            }
        } else {
            self.error_message = "Scene manager is not available.".to_string();
        }
    }

    /// Create a new entity under the selected scene
    fn create_new_entity(&mut self, entity_type: String, gui_state: &mut GuiState) {
        if let Some(scene_manager) = gui_state.scene_manager.as_mut() {
            let name = self.create_item_name.trim().to_string();
            if !name.is_empty() {
                // Check if the selected item is a scene, the entity must under a scene
                if let Some((item_type, scene_id)) = self.selected_item.as_ref() {
                    if item_type == "Scene" {
                        let scene_id = *scene_id;
                        if let Some(scene) = scene_manager.get_scene_mut(scene_id) {

                            let new_entity_id = scene.create_entity(&name);

                            // Add predefined attributes
                            if let Some(attributes) = PREDEFINED_ENTITIES.iter().find_map(|(predefined_name, attributes)| {
                                if predefined_name == &entity_type {
                                    Some(attributes)
                                } else {
                                    None
                                }
                            }) {
                                for (attr_name, attr_type, attr_value) in attributes.iter() {
                                    scene
                                        .get_entity_mut(new_entity_id)
                                        .unwrap()
                                        .create_attribute(attr_name, attr_type.clone(), attr_value.clone());
                                }
                            }

                            self.selected_item = Some(("Entity".to_string(), new_entity_id));
                            gui_state.selected_item = SelectedItem::Entity(scene_id, new_entity_id);

                            println!("Created new entity '{}' with type '{}' and ID: {:?}", name, entity_type, new_entity_id);

                            // Save project after creating the entity
                            if let Err(err) = ProjectManager::save_project_full(
                                Path::new(&gui_state.project_path),
                                gui_state.project_metadata.as_ref().unwrap(),
                                scene_manager,
                            ) {
                                self.error_message = format!("Error saving project after creating an entity: {}", err);
                            } else {
                                self.error_message.clear();
                            }
                        } else {
                            self.error_message = "The selected scene could not be found.".to_string();
                        }
                    } else {
                        self.error_message = "Please select a scene first to add the entity.".to_string();
                    }
                } else {
                    self.error_message = "Please select a scene first to add the entity.".to_string();
                }
            } else {
                self.error_message = "Entity name cannot be empty.".to_string();
            }
        } else {
            self.error_message = "Scene manager is not available.".to_string();
        }
    }

    /// Create a new resource under the selected scene
    fn create_new_resource(&mut self, gui_state: &mut GuiState) {
        if let Some(scene_manager) = gui_state.scene_manager.as_mut() {
            let name = self.create_item_name.trim().to_string();
            if !name.is_empty() {
                // Check if the selected item is a scene, the resource must under a scene
                if let Some((item_type, scene_id)) = self.selected_item.as_ref() {
                    if item_type == "Scene" {
                        let scene_id = *scene_id;
                        if let Some(scene) = scene_manager.get_scene_mut(scene_id) {

                            // Create an empty resource, empty path with Image type.
                            let new_resource_id = scene.create_resource(&name, "", ResourceType::Image);

                            self.selected_item = Some(("Resource".to_string(), new_resource_id));
                            gui_state.selected_item = SelectedItem::Resource(scene_id, new_resource_id);

                            println!("Created new resource '{}' with ID: {:?}", name, new_resource_id);

                            // Save project after creating the resource
                            if let Err(err) = ProjectManager::save_project_full(
                                Path::new(&gui_state.project_path),
                                gui_state.project_metadata.as_ref().unwrap(),
                                scene_manager,
                            ) {
                                self.error_message = format!("Error saving project after creating a resource: {}", err);
                            } else {
                                self.error_message.clear();
                            }
                        } else {
                            self.error_message = "The selected scene could not be found.".to_string();
                        }
                    } else {
                        self.error_message = "Please select a scene first to add the resource.".to_string();
                    }
                } else {
                    self.error_message = "Please select a scene first to add the resource.".to_string();
                }
            } else {
                self.error_message = "Resource name cannot be empty.".to_string();
            }
        } else {
            self.error_message = "Scene manager is not available.".to_string();
        }
    }

    /// Delete scene by scene id, save project after
    fn delete_scene(&mut self, scene_id: Uuid, gui_state: &mut GuiState) {
        if let Some(scene_manager) = gui_state.scene_manager.as_mut() {
            if scene_manager.delete_scene(scene_id) {
                println!("Deleted scene with ID: {:?}", scene_id);

                // Reset gui_state's selected item if the deleted scene was selected
                if matches!(gui_state.selected_item, SelectedItem::Scene(selected_id) if selected_id == scene_id) {
                    gui_state.selected_item = SelectedItem::None;
                }

                // Save project after deletion
                if let Err(err) = ProjectManager::save_project_full(
                    Path::new(&gui_state.project_path),
                    gui_state.project_metadata.as_ref().unwrap(),
                    scene_manager,
                ) {
                    println!("Error saving project after deleting a scene: {}", err);
                } else {
                    println!("Saved project after deleting a scene.");
                }
            } else {
                println!("Failed to delete the scene.");
            }
        }
    }

    /// Delete entity by given scene id and entity id, save project after
    fn delete_entity(&mut self, scene_id: Uuid, entity_id: Uuid, gui_state: &mut GuiState) {
        if let Some(scene_manager) = gui_state.scene_manager.as_mut() {
            if let Some(scene) = scene_manager.get_scene_mut(scene_id) {
                if scene.delete_entity(entity_id) {
                    println!("Deleted entity with ID: {:?}", entity_id);

                    // Reset gui_state's selected item if the deleted entity was selected
                    if matches!(
                        gui_state.selected_item,
                        SelectedItem::Entity(selected_scene, selected_entity)
                        if selected_scene == scene_id && selected_entity == entity_id
                    ) {
                        gui_state.selected_item = SelectedItem::None;
                    }

                    // Save project after deletion
                    if let Err(err) = ProjectManager::save_project_full(
                        Path::new(&gui_state.project_path),
                        gui_state.project_metadata.as_ref().unwrap(),
                        scene_manager,
                    ) {
                        println!("Error saving project after deleting an entity: {}", err);
                    } else {
                        println!("Saved project after deleting an entity.");
                    }
                } else {
                    println!("Failed to delete the entity.");
                }
            }
        }
    }

    /// Detach resource by given scene_id, entity id and resource id, save project after
    fn detach_resource(&mut self, scene_id: Uuid, entity_id: Uuid, resource_id: Uuid, gui_state: &mut GuiState) {
        if let Some(scene_manager) = gui_state.scene_manager.as_mut() {
            if let Some(scene) = scene_manager.get_scene_mut(scene_id) {
                if let Some(entity) = scene.get_entity_mut(entity_id) {
                    entity.detach_resource(resource_id);
                        println!("Detached resource for entity ID {:?} with resource ID: {:?}", entity_id, resource_id);

                        // Reset gui_state's selected item if the detached resource was selected
                        if matches!(
                            gui_state.selected_item,
                            SelectedItem::Resource(selected_scene, selected_resource)
                            if selected_resource == resource_id && selected_scene == scene_id
                        ) {
                            gui_state.selected_item = SelectedItem::None;
                        }

                        // Save project after deletion
                        if let Err(err) = ProjectManager::save_project_full(
                            Path::new(&gui_state.project_path),
                            gui_state.project_metadata.as_ref().unwrap(),
                            scene_manager,
                        ) {
                            println!("Error saving project after detach a resource: {}", err);
                        } else {
                            println!("Saved project after detach a resource.");
                        }

                } else {
                    println!("Failed to get the entity.");
                }
            } else {
                println!("Failed to get the scene.");
            }
        } else {
            println!("Failed to get the scene manager.");
        }
    }

    /// Rename popup
    fn open_rename_popup(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        title: &str,
        scene_id: Uuid,
        entity_id: Option<Uuid>,
        gui_state: &mut GuiState,
    ) {
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Enter new name:");
                ui.text_edit_singleline(&mut self.rename_input);

                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() {
                        if let Some(scene_manager) = gui_state.scene_manager.as_mut() {
                            if let Some(entity_id) = entity_id {
                                // Rename entity
                                if let Some(scene) = scene_manager.get_scene_mut(scene_id) {
                                    if let Some(entity) = scene.get_entity_mut(entity_id) {
                                        entity.name = self.rename_input.trim().to_string();
                                        println!("Renamed entity to: {}", entity.name);
                                    }
                                }
                            } else {
                                // Rename scene
                                if let Some(scene) = scene_manager.get_scene_mut(scene_id) {
                                    scene.name = self.rename_input.trim().to_string();
                                    println!("Renamed scene to: {}", scene.name);
                                }
                            }

                            // Save project after renaming
                            if let Err(err) = ProjectManager::save_project_full(
                                Path::new(&gui_state.project_path),
                                gui_state.project_metadata.as_ref().unwrap(),
                                scene_manager,
                            ) {
                                println!("Error saving project after renaming: {err}");
                            } else {
                                println!("Saved project after renaming.");
                            }
                        }

                        self.renaming_scene = None;
                        self.renaming_entity = None;
                        self.rename_input.clear();
                    }

                    if ui.button("Cancel").clicked() {
                        self.renaming_scene = None;
                        self.renaming_entity = None;
                        self.rename_input.clear();
                    }
                });
            });
    }
}
