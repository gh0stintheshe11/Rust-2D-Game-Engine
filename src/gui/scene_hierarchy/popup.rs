use crate::ecs::{PhysicsProperties, ResourceType};
use crate::gui::gui_state::{GuiState, ScenePanelSelectedItem, SelectedItem};
use crate::gui::scene_hierarchy::predefined_entities::PREDEFINED_ENTITIES;
use crate::gui::scene_hierarchy::utils;
use crate::project_manager::ProjectManager;
use eframe::egui::{Context, Ui};
use std::path::Path;
use uuid::Uuid;


pub struct PopupManager {
    pub create_item_name: String,
    pub error_message: String,
    pub rename_input: String,
    pub scene_rename_scene: Option<Uuid>,
    pub entity_rename_entity: Option<(Uuid, Uuid)>,
    pub create_popup_active: bool,
    pub create_item_type: String,
    pub manage_resource_popup_active: bool,
    pub manage_resources_entity: Option<(Uuid, Uuid)>, // Track selected entity for manage resource (scene_id, entity_id)
    pub selected_resources: std::collections::HashMap<Uuid, bool>,
    pub manage_scene_resources_active: bool,
    pub manage_resources_scene: Option<Uuid>,// Track selected scene for manage resource (scene_id)
    pub resource_rename_inputs: std::collections::HashMap<Uuid, String>,
    pub resource_editing_states: std::collections::HashMap<Uuid, bool>, // Track editing states for each resource
    pub create_resource_type: ResourceType,
}

impl PopupManager {
    pub fn new() -> Self {
        Self {
            create_item_name: String::new(),
            error_message: String::new(),
            rename_input: String::new(),
            scene_rename_scene: None,
            entity_rename_entity: None,
            create_popup_active: false,
            create_item_type: "Scene".to_string(),
            manage_resource_popup_active: false,
            manage_resources_entity: None,
            selected_resources: std::collections::HashMap::new(),
            manage_scene_resources_active: false,
            manage_resources_scene: None,
            resource_rename_inputs: std::collections::HashMap::new(),
            resource_editing_states: std::collections::HashMap::new(),
            create_resource_type: ResourceType::Image,
        }
    }

    pub fn start_rename_scene(&mut self, scene_id: Uuid, current_name: String) {
        self.scene_rename_scene = Some(scene_id);
        self.rename_input = current_name;
    }

    pub fn reset_rename_scene(&mut self) {
        self.scene_rename_scene = None;
        self.rename_input.clear();
        self.error_message.clear();
    }

    pub fn start_rename_entity(&mut self, scene_id: Uuid, entity_id: Uuid, current_name: String) {
        self.entity_rename_entity = Some((scene_id, entity_id));
        self.rename_input = current_name;
    }

    pub fn reset_rename(&mut self) {
        self.scene_rename_scene = None;
        self.entity_rename_entity = None;
        self.rename_input.clear();
        self.error_message.clear();
    }

    pub fn start_manage_scene_resources(&mut self, scene_id: Uuid) {
        self.manage_scene_resources_active = true;
        self.manage_resources_scene = Some(scene_id);
        self.resource_rename_inputs.clear();
        self.resource_editing_states.clear();
        self.error_message.clear();
    }

    pub fn reset_manage_scene_resources(&mut self) {
        self.manage_scene_resources_active = false;
        self.manage_resources_scene = None;
        self.resource_rename_inputs.clear();
        self.resource_editing_states.clear();
        self.error_message.clear();
    }

    /// Rename popup, for both scene and entity
    pub fn render_rename_popup(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        gui_state: &mut GuiState,
    ) {

        let (title, scene_id, entity_id) = match self.scene_rename_scene {
            Some(scene_id) => ("Rename Scene", scene_id, None),
            None => match self.entity_rename_entity {
                Some((scene_id, entity_id)) => ("Rename Entity", scene_id, Some(entity_id)),
                None => return,
            },
        };

        // Render rename popup
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Enter new name:");
                ui.text_edit_singleline(&mut self.rename_input);

                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() {
                        self.rename_item(scene_id, entity_id, gui_state);
                    }
                    if ui.button("Cancel").clicked() {
                        self.reset_rename();
                    }
                });
            });
    }

    /// Handle renaming
    fn rename_item(&mut self, scene_id: Uuid, entity_id: Option<Uuid>, gui_state: &mut GuiState) {
        if let Some(scene_manager) = gui_state.scene_manager.as_mut() {
            let new_name = self.rename_input.trim().to_string();
            if let Some(entity_id) = entity_id {
                // Rename entity
                if let Some(entity) = scene_manager
                    .get_scene_mut(scene_id)
                    .and_then(|scene| scene.get_entity_mut(entity_id))
                {
                    entity.name = new_name;
                    println!("Renamed entity to: {}", entity.name);
                }
            } else {
                // Rename scene
                if let Some(scene) = scene_manager.get_scene_mut(scene_id) {
                    scene.name = new_name;
                    println!("Renamed scene to: {}", scene.name);
                }
            }

            // Save project
            utils::save_project(gui_state);

        }

        self.reset_rename();
    }

    /// Render create popup in panel
    pub fn render_create_popup(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        gui_state: &mut GuiState,
    ) {
        egui::Window::new("Create New Node")
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.label("Select item to create:");

                ui.vertical(|ui| {
                    let available_width = ui.available_width();

                    let all_item_types = [
                        ("Scene", "Scene"),
                        ("Entity", "Entity"),
                        ("Resource", "Resource"),
                        ("Camera", "Camera"),
                        ("Physics", "Physics"),
                    ];

                    for (type_name, label) in all_item_types {
                        let is_selected = self.create_item_type == type_name;

                        if ui
                            .add_sized(
                                [available_width, 24.0],
                                egui::SelectableLabel::new(is_selected, label),
                            )
                            .clicked()
                        {
                            self.create_item_type = type_name.to_string();
                        }
                    }
                });

                ui.add_space(10.0);

                if self.create_item_type == "Resource" {
                    ui.separator();
                    ui.label("Resource Type:");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.create_resource_type, ResourceType::Image, "Image");
                        ui.selectable_value(&mut self.create_resource_type, ResourceType::Sound, "Sound");
                        ui.selectable_value(&mut self.create_resource_type, ResourceType::Script, "Script");
                        // Add more resource types as needed
                    });
                }

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.create_item_name);
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        self.handle_create(gui_state);
                    }

                    if ui.button("Cancel").clicked() {
                        self.reset_create_popup();
                    }
                });

                if !self.error_message.is_empty() {
                    ui.add_space(10.0);
                    ui.colored_label(egui::Color32::RED, &self.error_message);
                }
            });
    }

    fn handle_create(&mut self, gui_state: &mut GuiState) {
        if self.create_item_name.trim().is_empty() {
            self.error_message = "Please select an item type and enter a valid name.".to_string();
            return;
        }

        match self.create_item_type.as_str() {
            "Scene" => self.create_new_scene(gui_state),
            "Entity" => self.create_new_entity("Entity".to_string(), gui_state, "Empty"),
            "Resource" => self.create_new_resource(gui_state),
            "Camera" => self.create_new_entity("Entity".to_string(), gui_state, "Camera"),
            "Physics" => self.create_new_entity("Entity".to_string(), gui_state, "Physics"),
            other => {
                // Predefined entity creation
                // if PREDEFINED_ENTITIES
                //     .iter()
                //     .any(|entity| entity.name == other)
                // {
                //     self.create_new_entity(other.to_string(), gui_state);
                // }
            }
        }

        if self.error_message.is_empty() {
            self.reset_create_popup();
        }
    }

    fn reset_create_popup(&mut self) {
        self.create_popup_active = false;
        self.create_item_name.clear();
        self.error_message.clear();
        self.create_resource_type = ResourceType::Image; // Reset to default type
    }

    /// Create a new scene
    fn create_new_scene(&mut self, gui_state: &mut GuiState) {
        // Ensure scene manager exists
        let scene_manager = match &mut gui_state.scene_manager {
            Some(manager) => manager,
            None => {
                self.error_message = "Scene manager is not available.".to_string();
                return;
            }
        };

        // Ensure scene name is not empty
        let name = self.create_item_name.trim();
        if name.is_empty() {
            self.error_message = "Scene name cannot be empty.".to_string();
            return;
        }

        // Create the new scene
        let new_scene_id = scene_manager.create_scene(name);

        // Update selection state
        gui_state.scene_panel_selected_item = ScenePanelSelectedItem::Scene(new_scene_id);
        gui_state.selected_item = SelectedItem::Scene(new_scene_id);

        println!("Created new scene '{}' with ID: {:?}", name, new_scene_id);

        // Save the project
        utils::save_project(gui_state);
        self.reset_create_popup();
    }

    /// Create a new entity under the selected scene
    fn create_new_entity(&mut self, entity_type: String, gui_state: &mut GuiState, predefined_type: &str) {
        // Ensure scene manager exists
        let scene_manager = match &mut gui_state.scene_manager {
            Some(manager) => manager,
            None => {
                self.error_message = "Scene manager is not available.".to_string();
                return;
            }
        };

        // Ensure entity name is not empty
        let name = self.create_item_name.trim();
        if name.is_empty() {
            self.error_message = "Entity name cannot be empty.".to_string();
            return;
        }

        let scene_id = match &gui_state.scene_panel_selected_item {
            ScenePanelSelectedItem::Scene(scene_id) => scene_id,
            _ => {
                println!("Selected item is not a Scene.");
                self.error_message = "Please select a scene first to add the entity.".to_string();
                return;
            }
        };

        // Get the selected scene
        let scene = match scene_manager.get_scene_mut(*scene_id) {
            Some(scene) => scene,
            None => {
                self.error_message = "The selected scene could not be found.".to_string();
                return;
            }
        };

        // Create the new entity
        let new_entity_id = match predefined_type {
            "Empty" => scene.create_entity(name),
            "Camera" => scene.create_camera(name),
            "Physics" => scene.create_physical_entity(name, (0.0, 0.0), PhysicsProperties::default()),
            _ => {
                println!("Unknown predefined type: {}", predefined_type);
                return;
            }
        };


        // Add predefined attributes
        // if let Some(predefined_entity) = PREDEFINED_ENTITIES
        //     .iter()
        //     .find(|entity| entity.name == entity_type)
        // {
        //     for (attr_name, attr_type, attr_value) in predefined_entity.attributes {
        //         scene
        //             .get_entity_mut(new_entity_id)
        //             .unwrap()
        //             .create_attribute(attr_name, attr_type.clone(), attr_value.clone());
        //     }
        // }

        let scene_id = match &gui_state.scene_panel_selected_item {
            ScenePanelSelectedItem::Scene(scene_id) => *scene_id,
            _ => {
                println!("Selected item is not a Scene.");
                self.error_message = "Please select a scene first to add the entity.".to_string();
                return;
            }
        };

        // Update selection
        gui_state.scene_panel_selected_item = ScenePanelSelectedItem::Entity(scene_id, new_entity_id);
        gui_state.selected_item = SelectedItem::Entity(scene_id, new_entity_id);

        println!(
            "Created new entity '{}' with type '{}' and ID: {:?}",
            name, entity_type, new_entity_id
        );

        // Save the project
        utils::save_project(gui_state);
        self.reset_create_popup();
    }

    /// Create a new resource under the selected scene
    fn create_new_resource(&mut self, gui_state: &mut GuiState) {
        // Ensure a scene is selected
        let scene_id = match &gui_state.scene_panel_selected_item {
            ScenePanelSelectedItem::Scene(scene_id) => *scene_id,
            _ => {
                self.error_message = "Please select a scene first to add the resource.".to_string();
                return;
            }
        };

        // Ensure resource name is not empty
        let name = self.create_item_name.trim();
        if name.is_empty() {
            self.error_message = "Resource name cannot be empty.".to_string();
            return;
        }

        // Ensure scene manager exists
        let scene_manager = match &mut gui_state.scene_manager {
            Some(manager) => manager,
            None => {
                self.error_message = "Scene manager is not available.".to_string();
                return;
            }
        };

        // Get the selected scene
        let scene = match scene_manager.get_scene_mut(scene_id) {
            Some(scene) => scene,
            None => {
                self.error_message = "The selected scene could not be found.".to_string();
                return;
            }
        };

        // Create the new resource with the name as the path and selected type
        let new_resource_id = scene.create_resource(name, name, self.create_resource_type.clone());

        // Update selection state
        gui_state.scene_panel_selected_item = ScenePanelSelectedItem::Resource(scene_id, new_resource_id);
        gui_state.selected_item = SelectedItem::Resource(scene_id, new_resource_id);

        println!(
            "Created new resource '{}' of type {:?} with ID: {:?}",
            name, self.create_resource_type, new_resource_id
        );

        // Save the project
        utils::save_project(gui_state);
        self.reset_create_popup();
    }

    pub fn render_manage_resources_popup(
        &mut self,
        ctx: &Context,
        ui: &mut Ui,
        gui_state: &mut GuiState,
        scene_id: Uuid,
        entity_id: Uuid,
    ) {
        if self.manage_resource_popup_active {
            let mut resource_updated = false;

            egui::Window::new("Manage Resources")
                .collapsible(false)
                .resizable(false)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    let scene_manager = match gui_state.scene_manager.as_ref() {
                        Some(manager) => manager,
                        None => {
                            ui.label("No scene manager available.");
                            return;
                        }
                    };

                    let scene = match scene_manager.get_scene(scene_id) {
                        Some(scene) => scene,
                        None => {
                            ui.label("Scene not found.");
                            return;
                        }
                    };

                    let entity = match scene.get_entity(entity_id) {
                        Some(entity) => entity,
                        None => {
                            ui.label("Entity not found.");
                            return;
                        }
                    };

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
                        }
                        if ui.button("Cancel").clicked() {
                            self.reset_manage_resources_popup();
                        }
                    });

                    if resource_updated {
                        if let Some(scene_manager) = &mut gui_state.scene_manager {
                            if let Some(entity) = scene_manager
                                .get_scene_mut(scene_id)
                                .and_then(|scene| scene.get_entity_mut(entity_id))
                            {
                                // Update the entity's resource list based on selection
                                entity.resource_list =
                                    self.selected_resources
                                        .iter()
                                        .filter_map(|(resource_id, &selected)| {
                                            if selected {
                                                Some(*resource_id)
                                            } else {
                                                None
                                            }
                                        })
                                        .collect();

                                // Save the project after updating
                                utils::save_project(gui_state);
                                self.reset_manage_resources_popup();
                            } else {
                                println!("Error: Entity or scene not found.");
                            }
                        } else {
                            println!("Error: Scene manager not available.");
                        }
                    }
                });
        }
    }

    fn reset_manage_resources_popup(&mut self) {
        self.manage_resource_popup_active = false;
        self.selected_resources.clear();
        self.error_message.clear();
    }

    pub fn render_popups(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        // Render rename popup
        self.render_rename_popup(ctx, ui, gui_state);

        // Render create popup
        if self.create_popup_active {
            self.render_create_popup(ctx, ui, gui_state);
        }

        // Render manage resources popup
        if self.manage_resource_popup_active {
            if let Some((scene_id, entity_id)) = self.manage_resources_entity {
                self.render_manage_resources_popup(ctx, ui, gui_state, scene_id, entity_id);
            }
        }

        // Render manage resources popup for scenes
        if self.manage_scene_resources_active {
            if let Some(scene_id) = self.manage_resources_scene {
                self.render_manage_scene_resources_popup(ctx, ui, gui_state, scene_id);
            } else {
                println!("Error: Scene ID is not set for managing resources.");
            }
        }

    }

    /// Manage resources for scene, allow rename and delete. Check if resource linked to a entity before delete.
    pub fn render_manage_scene_resources_popup(
        &mut self,
        ctx: &Context,
        ui: &mut Ui,
        gui_state: &mut GuiState,
        scene_id: Uuid,
    ) {
        if !self.manage_scene_resources_active {
            return;
        }


        egui::Window::new("Manage Scene Resources")
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                // Get resources and entities outside, avoid borrow issues.
                let (resources, entities): (Vec<(Uuid, String)>, Vec<Uuid>) = match gui_state
                    .scene_manager
                    .as_ref()
                    .and_then(|scene_manager| scene_manager.get_scene(scene_id))
                    .map(|scene| {
                        let resources: Vec<(Uuid, String)> = scene
                            .resources
                            .iter()
                            .map(|(id, res)| (*id, res.name.clone()))
                            .collect();
                        let entities: Vec<Uuid> = scene
                            .entities
                            .values()
                            .flat_map(|entity| entity.resource_list.iter().copied())
                            .collect();
                        (resources, entities)
                    }) {
                    Some(data) => data,
                    None => {
                        ui.label("No scene manager or scene available.");
                        return;
                    }
                };

                let max_height = ctx.available_rect().height() * 0.6;

                // Display resources
                egui::ScrollArea::vertical()
                    .max_height(max_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for (resource_id, resource_name) in resources {
                            let editing = self.resource_editing_states.entry(resource_id).or_insert(false);
                            let rename_input = self
                                .resource_rename_inputs
                                .entry(resource_id)
                                .or_insert(resource_name.clone());

                            ui.horizontal(|ui| {
                                if *editing {
                                    ui.text_edit_singleline(rename_input);
                                    if ui.button("Save").clicked() {
                                        if let Some(scene_manager) = gui_state.scene_manager.as_mut() {
                                            if let Some(scene) = scene_manager.get_scene_mut(scene_id) {
                                                if let Some(resource) = scene.get_resource_mut(resource_id) {
                                                    resource.name = rename_input.clone();
                                                    println!("Renamed resource to: {}", resource.name);
                                                }
                                            }
                                            utils::save_project(gui_state);
                                        }
                                        *editing = false;
                                    }
                                } else {
                                    ui.label(&resource_name);
                                    if ui.button("Rename").clicked() {
                                        *editing = true;
                                    }
                                }

                                if ui.button("Delete").clicked() {
                                    let is_linked = entities.contains(&resource_id);

                                    if is_linked {
                                        self.error_message = format!(
                                            "Cannot delete resource '{}' because it is linked to an entity.",
                                            resource_name
                                        );
                                    } else if let Some(scene_manager) = gui_state.scene_manager.as_mut() {
                                        if let Some(scene) = scene_manager.get_scene_mut(scene_id) {
                                            scene.delete_resource(resource_id);
                                            println!("Deleted resource with ID: {:?}", resource_id);
                                            utils::save_project(gui_state);
                                        }
                                    }
                                }
                            });
                        }
                    });

                if !self.error_message.is_empty() {
                    ui.add_space(10.0);
                    ui.colored_label(egui::Color32::RED, &self.error_message);
                }

                ui.separator();

                if ui.button("Close").clicked() {
                    self.reset_manage_scene_resources();
                }
            });

    }

}
