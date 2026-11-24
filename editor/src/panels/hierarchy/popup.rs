use crate::state::GuiState;
use rust_2d_game_engine::logger::LOGGER;
use rust_2d_game_engine::scene::SceneManager;
use std::path::Path;
use uuid::Uuid;

/// What the rename popup renames.
#[derive(Clone, Copy)]
pub enum RenameTarget {
    Scene(Uuid),
    Entity(Uuid, Uuid), // scene id, entity id
}

/// A destructive action awaiting user confirmation.
pub enum PendingDelete {
    Scene(Uuid, String),        // scene id, display name
    Entity(Uuid, Uuid, String), // scene id, entity id, display name
}

pub struct PopupManager {
    pub create_item_name: String,
    pub error_message: String,
    pub rename_input: String,
    pub rename_target: Option<RenameTarget>,
    pub create_popup_active: bool,
    pub create_item_type: String,
    pub manage_assets_entity: Option<(Uuid, Uuid)>,
    pub manage_assets_popup_active: bool,
    pub resource_selection: Option<(Uuid, Uuid)>,
    pub resource_selection_popup_active: bool,
    pub selected_resource_type: String,
    pub available_resources: Vec<std::path::PathBuf>,
    // Which directory available_resources was scanned from (cache key)
    available_resources_path: Option<std::path::PathBuf>,
    pub pending_delete: Option<PendingDelete>,
    // Set by the attach/detach popups when they mutate an entity; consumed
    // by render_popups to persist the project
    assets_changed: bool,
}

impl Default for PopupManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PopupManager {
    pub fn new() -> Self {
        Self {
            create_item_name: String::new(),
            error_message: String::new(),
            rename_input: String::new(),
            rename_target: None,
            create_popup_active: false,
            create_item_type: "Scene".to_string(),
            manage_assets_entity: None,
            manage_assets_popup_active: false,
            resource_selection: None,
            resource_selection_popup_active: false,
            selected_resource_type: "Images".to_string(),
            available_resources: Vec::new(),
            available_resources_path: None,
            pending_delete: None,
            assets_changed: false,
        }
    }

    pub fn start_rename(&mut self, target: RenameTarget, current_name: String) {
        self.rename_target = Some(target);
        self.rename_input = current_name;
        self.error_message.clear();
    }

    pub fn reset_rename(&mut self) {
        self.rename_target = None;
        self.rename_input.clear();
        self.error_message.clear();
    }

    /// Close every popup (e.g. when another project is opened).
    pub fn close_all(&mut self) {
        *self = Self::new();
    }

    /// Rename popup, for both scenes and entities
    pub fn render_rename_popup(
        &mut self,
        ctx: &egui::Context,
        _ui: &mut egui::Ui,
        gui_state: &mut GuiState,
    ) {
        let Some(target) = self.rename_target else {
            return;
        };
        let title = match target {
            RenameTarget::Scene(_) => "Rename Scene",
            RenameTarget::Entity(_, _) => "Rename Entity",
        };

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.label("Enter new name:");
                let response = ui.text_edit_singleline(&mut self.rename_input);
                let submitted =
                    response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() || submitted {
                        self.rename_item(target, gui_state);
                    }
                    if ui.button("Cancel").clicked() {
                        self.reset_rename();
                    }
                });
                if !self.error_message.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.error_message);
                }
            });
    }

    /// Apply the rename; an empty name keeps the popup open with an error.
    fn rename_item(&mut self, target: RenameTarget, gui_state: &mut GuiState) {
        let new_name = self.rename_input.trim().to_string();
        let Some(scene_manager) = gui_state.scene_manager.as_mut() else {
            self.reset_rename();
            return;
        };
        let result = match target {
            RenameTarget::Scene(scene_id) => scene_manager
                .get_scene_mut(scene_id)
                .ok_or_else(|| "The scene no longer exists".to_string())
                .and_then(|scene| {
                    let changed = scene.name != new_name;
                    scene.modify_scene(&new_name).map(|()| changed)
                }),
            RenameTarget::Entity(scene_id, entity_id) => scene_manager
                .get_scene_mut(scene_id)
                .ok_or_else(|| "The scene no longer exists".to_string())
                .and_then(|scene| scene.get_entity_mut(entity_id))
                .and_then(|entity| {
                    let changed = entity.name != new_name;
                    entity.change_entity_name(&new_name).map(|()| changed)
                }),
        };
        match result {
            Ok(changed) => {
                if changed {
                    gui_state.commit_change();
                }
                self.reset_rename();
            }
            Err(e) => self.error_message = e,
        }
    }

    /// Render create popup in panel
    pub fn render_create_popup(
        &mut self,
        ctx: &egui::Context,
        _ui: &mut egui::Ui,
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
                        ("Camera", "Camera"),
                        ("Physics", "Physics"),
                    ];

                    for (type_name, label) in all_item_types {
                        let is_selected = self.create_item_type == type_name;

                        if ui
                            .add_sized(
                                [available_width, 24.0],
                                egui::Button::selectable(is_selected, label),
                            )
                            .clicked()
                        {
                            self.create_item_type = type_name.to_string();
                        }
                    }
                });

                ui.add_space(10.0);

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

        let item_type = self.create_item_type.clone();
        match item_type.as_str() {
            "Scene" => self.create_new_scene(gui_state),
            "Entity" => self.create_new_entity(gui_state, "Empty"),
            "Camera" => self.create_new_entity(gui_state, "Camera"),
            "Physics" => self.create_new_entity(gui_state, "Physics"),
            _ => {}
        }

        if self.error_message.is_empty() {
            self.reset_create_popup();
        }
    }

    fn reset_create_popup(&mut self) {
        self.create_popup_active = false;
        self.create_item_name.clear();
        self.error_message.clear();
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
        match scene_manager.create_scene(name) {
            Ok(new_scene_id) => {
                gui_state.select_scene(new_scene_id);
                // Save the project
                gui_state.commit_change();
                self.reset_create_popup();
            }
            Err(e) => {
                self.error_message = format!("Failed to create scene: {}", e);
            }
        }
    }

    /// Create a new entity under the selected scene
    fn create_new_entity(&mut self, gui_state: &mut GuiState, predefined_type: &str) {
        let name = self.create_item_name.trim().to_string();
        if name.is_empty() {
            self.error_message = "Entity name cannot be empty.".to_string();
            return;
        }
        let Some(scene_id) = gui_state.selected_scene() else {
            self.error_message = "Select a scene (or one of its entities) first.".to_string();
            return;
        };
        let Some(scene) = gui_state
            .scene_manager
            .as_mut()
            .and_then(|manager| manager.get_scene_mut(scene_id))
        else {
            self.error_message = "The selected scene could not be found.".to_string();
            return;
        };

        let new_entity_id = match predefined_type {
            "Camera" => scene.create_camera(&name),
            "Physics" => scene.create_entity(&name).inspect(|entity_id| {
                if let Ok(entity) = scene.get_entity_mut(*entity_id) {
                    entity.add_physics_attributes();
                }
            }),
            _ => scene.create_entity(&name),
        };

        match new_entity_id {
            Ok(entity_id) => {
                gui_state.select_entity(scene_id, entity_id);
                gui_state.commit_change();
                self.reset_create_popup();
            }
            Err(e) => {
                self.error_message = format!("Failed to create entity: {}", e);
            }
        }
    }

    pub fn render_popups(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        gui_state: &mut GuiState,
    ) {
        // Render rename popup
        self.render_rename_popup(ctx, ui, gui_state);

        // Render create popup
        if self.create_popup_active {
            self.render_create_popup(ctx, ui, gui_state);
        }

        // Render manage assets popup
        if self.manage_assets_popup_active
            && let Some(scene_manager) = &mut gui_state.scene_manager
        {
            self.show_manage_assets_popup(ctx, scene_manager);
        }

        // Render resource selection popup
        if self.resource_selection.is_some()
            && let Some(scene_manager) = &mut gui_state.scene_manager
        {
            self.show_resource_selection_popup(ctx, scene_manager, &gui_state.project_path);
        }

        // Persist attach/detach changes made by the popups above
        if self.assets_changed {
            self.assets_changed = false;
            gui_state.commit_change();
        }

        // Render delete confirmation
        self.render_delete_confirmation(ctx, gui_state);
    }

    fn render_delete_confirmation(&mut self, ctx: &egui::Context, gui_state: &mut GuiState) {
        let Some(pending) = &self.pending_delete else {
            return;
        };

        let (kind, name) = match pending {
            PendingDelete::Scene(_, name) => ("scene", name.clone()),
            PendingDelete::Entity(_, _, name) => ("entity", name.clone()),
        };

        let mut close = false;
        egui::Window::new("Confirm Delete")
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(format!("Delete {} '{}'?", kind, name));
                ui.weak("Edit > Undo (Ctrl+Z) brings it back.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Delete").clicked() {
                        match self.pending_delete.take() {
                            Some(PendingDelete::Scene(scene_id, _)) => {
                                if let Some(scene_manager) = &mut gui_state.scene_manager {
                                    match scene_manager.delete_scene(scene_id) {
                                        Ok(_) => {
                                            gui_state.clear_selection();
                                            gui_state.commit_change();
                                        }
                                        Err(e) => {
                                            LOGGER.error(format!("Failed to delete scene: {}", e))
                                        }
                                    }
                                }
                            }
                            Some(PendingDelete::Entity(scene_id, entity_id, _)) => {
                                let deleted = gui_state
                                    .scene_manager
                                    .as_mut()
                                    .and_then(|manager| manager.get_scene_mut(scene_id))
                                    .map(|scene| scene.delete_entity(entity_id));
                                match deleted {
                                    Some(Ok(true)) => {
                                        gui_state.clear_selection();
                                        gui_state.commit_change();
                                    }
                                    Some(Ok(false)) => {}
                                    Some(Err(e)) => {
                                        LOGGER.error(format!("Failed to delete entity: {}", e))
                                    }
                                    None => {
                                        LOGGER.error("Failed to delete entity: scene not found")
                                    }
                                }
                            }
                            None => {}
                        }
                        close = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });

        if close {
            self.pending_delete = None;
        }
    }

    pub fn show_resource_selection_popup(
        &mut self,
        ctx: &egui::Context,
        scene_manager: &mut SceneManager,
        project_path: &Path,
    ) {
        if let Some((scene_id, entity_id)) = self.resource_selection {
            let mut changed = false;
            egui::Window::new("Attach Resource")
                .open(&mut self.resource_selection_popup_active)
                .collapsible(false)
                .resizable(false)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    // Resource type dropdown
                    egui::CollapsingHeader::new("Resource Type")
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::ComboBox::from_label("")
                                .selected_text(&self.selected_resource_type)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.selected_resource_type,
                                        "Images".to_string(),
                                        "Images",
                                    );
                                    ui.selectable_value(
                                        &mut self.selected_resource_type,
                                        "Sounds".to_string(),
                                        "Sounds",
                                    );
                                    ui.selectable_value(
                                        &mut self.selected_resource_type,
                                        "Scripts".to_string(),
                                        "Scripts",
                                    );
                                });
                        });

                    // Update available resources when the type changes (or on
                    // first open) instead of re-reading the directory every frame
                    let resource_path = match self.selected_resource_type.as_str() {
                        "Images" => project_path.join("assets").join("images"),
                        "Sounds" => project_path.join("assets").join("sounds"),
                        "Scripts" => project_path.join("assets").join("scripts"),
                        _ => project_path.join("assets"),
                    };

                    if self.available_resources_path.as_deref() != Some(resource_path.as_path()) {
                        self.available_resources = std::fs::read_dir(&resource_path)
                            .map(|entries| {
                                entries.filter_map(|e| e.ok()).map(|e| e.path()).collect()
                            })
                            .unwrap_or_default();
                        self.available_resources_path = Some(resource_path);
                    }

                    // Show resource list in a collapsing header
                    egui::CollapsingHeader::new("Available Resources")
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                for resource_path in &self.available_resources {
                                    if let Some(filename) = resource_path.file_name()
                                        && ui
                                            .button(filename.to_string_lossy().to_string())
                                            .clicked()
                                        && let Some(scene) = scene_manager.get_scene_mut(scene_id)
                                        && let Ok(entity) = scene.get_entity_mut(entity_id)
                                    {
                                        match self.selected_resource_type.as_str() {
                                            "Images" => {
                                                // Check if image is already attached
                                                if !entity.images.contains(resource_path) {
                                                    entity.images.push(resource_path.clone());
                                                    changed = true;
                                                }
                                            }
                                            "Sounds" => {
                                                // Check if sound is already attached
                                                if !entity.sounds.contains(resource_path) {
                                                    entity.sounds.push(resource_path.clone());
                                                    changed = true;
                                                }
                                            }
                                            "Scripts"
                                                if entity.script.as_ref()
                                                    != Some(resource_path) =>
                                            {
                                                entity.script = Some(resource_path.clone());
                                                changed = true;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            });
                        });
                });

            // If window is closed, reset the selection
            if !self.resource_selection_popup_active {
                self.resource_selection = None;
                // Rescan on next open so new imports show up
                self.available_resources_path = None;
            }

            self.assets_changed |= changed;
        }
    }

    pub fn show_manage_assets_popup(
        &mut self,
        ctx: &egui::Context,
        scene_manager: &mut SceneManager,
    ) {
        if let Some((scene_id, entity_id)) = self.manage_assets_entity {
            let mut changed = false;
            egui::Window::new("Manage Assets")
                .open(&mut self.manage_assets_popup_active)
                .collapsible(false)
                .resizable(false)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    if let Some(scene) = scene_manager.get_scene_mut(scene_id)
                        && let Ok(entity) = scene.get_entity_mut(entity_id)
                    {
                        // Images section as collapsing header
                        if !entity.images.is_empty() {
                            egui::CollapsingHeader::new("Images")
                                .default_open(true)
                                .show(ui, |ui| {
                                    let mut images_to_remove = Vec::new();
                                    for (i, path) in entity.images.iter().enumerate() {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                path.file_name()
                                                    .unwrap_or_default()
                                                    .to_string_lossy()
                                                    .to_string(),
                                            );
                                            if ui.button("Remove").clicked() {
                                                images_to_remove.push(i);
                                            }
                                        });
                                    }
                                    // Remove images outside the loop
                                    for &i in images_to_remove.iter().rev() {
                                        entity.images.remove(i);
                                        changed = true;
                                    }
                                });
                        }

                        // Sounds section as collapsing header
                        if !entity.sounds.is_empty() {
                            egui::CollapsingHeader::new("Sounds")
                                .default_open(true)
                                .show(ui, |ui| {
                                    let mut sounds_to_remove = Vec::new();
                                    for (i, path) in entity.sounds.iter().enumerate() {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                path.file_name()
                                                    .unwrap_or_default()
                                                    .to_string_lossy()
                                                    .to_string(),
                                            );
                                            if ui.button("Remove").clicked() {
                                                sounds_to_remove.push(i);
                                            }
                                        });
                                    }
                                    // Remove sounds outside the loop
                                    for &i in sounds_to_remove.iter().rev() {
                                        entity.sounds.remove(i);
                                        changed = true;
                                    }
                                });
                        }

                        // Script section as collapsing header
                        if entity.script.is_some() {
                            egui::CollapsingHeader::new("Script")
                                .default_open(true)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            entity
                                                .script
                                                .clone()
                                                .unwrap()
                                                .file_name()
                                                .unwrap_or_default()
                                                .to_string_lossy()
                                                .to_string(),
                                        );
                                        if ui.button("Remove").clicked() {
                                            entity.script = None;
                                            changed = true;
                                        }
                                    });
                                });
                        }
                    }
                });

            self.assets_changed |= changed;
        }
    }
}
