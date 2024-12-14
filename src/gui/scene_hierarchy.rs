use eframe::egui;
use crate::gui::gui_state::GuiState;
use uuid::Uuid;
use crate::project_manager::ProjectManager;
use std::path::Path;

pub struct SceneHierarchy {
    search_query: String,
    show_create_popup: bool,
    create_item_type: String,
    create_item_name: String,
    selected_item: Option<(String, Uuid)>, // Track selected item (type, ID) (type: Scene, Entity)
    error_message: String,
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

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
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

                                if ui
                                    .selectable_label(selected, &scene.name)
                                    .clicked()
                                {
                                    self.selected_item = Some(("Scene".to_string(), *scene_id));
                                }
                            })
                            .body(|ui| {
                                for (entity_id, entity) in &scene.entities {
                                    // Apply search filter to entities
                                    if !self.search_query.is_empty()
                                        && !entity.name.to_lowercase().contains(&self.search_query.to_lowercase())
                                    {
                                        continue;
                                    }

                                    let selected = self
                                        .selected_item
                                        .as_ref()
                                        .map_or(false, |(item_type, id)| item_type == "Entity" && id == entity_id);

                                    if ui
                                        .selectable_label(selected, format!("📌 {}", entity.name))
                                        .clicked()
                                    {
                                        self.selected_item = Some(("Entity".to_string(), *entity_id));
                                    }
                                }
                            });
                    }
                } else {
                    ui.label("No scenes loaded.");
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
                                        self.create_new_entity(gui_state);
                                        if self.error_message.is_empty() {
                                            self.show_create_popup = false;
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

    /// Create a new scene
    fn create_new_scene(&mut self, gui_state: &mut GuiState) {
        if let Some(scene_manager) = gui_state.scene_manager.as_mut() {
            let name = self.create_item_name.trim().to_string();
            if !name.is_empty() {
                let new_scene_id = scene_manager.create_scene(&name);
                self.selected_item = Some(("Scene".to_string(), new_scene_id));
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
    fn create_new_entity(&mut self, gui_state: &mut GuiState) {
        if let Some(scene_manager) = gui_state.scene_manager.as_mut() {
            let name = self.create_item_name.trim().to_string();
            if !name.is_empty() {
                // Check if the selected item is a scene
                if let Some((item_type, scene_id)) = self.selected_item.as_ref() {
                    if item_type == "Scene" {
                        if let Some(scene) = scene_manager.get_scene_mut(*scene_id) {
                            let new_entity_id = scene.create_entity(&name);
                            self.selected_item = Some(("Entity".to_string(), new_entity_id));
                            println!("Created new entity with ID: {:?}", new_entity_id);

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
}
