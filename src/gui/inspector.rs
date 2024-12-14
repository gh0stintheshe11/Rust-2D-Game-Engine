use eframe::egui;
use crate::gui::gui_state::{GuiState, SelectedItem};
use std::path::Path;
use uuid::Uuid;

pub struct Inspector;

impl Inspector {
    pub fn new() -> Self {
        Self
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &GuiState) {
        match &gui_state.selected_item {
            SelectedItem::Scene(scene_id) => self.show_scene_details(ui, *scene_id, gui_state),
            SelectedItem::Entity(scene_id, entity_id) => {
                self.show_entity_details(ui, *scene_id, *entity_id, gui_state)
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

    // Display entity information
    fn show_entity_details(
        &self,
        ui: &mut egui::Ui,
        scene_id: Uuid,
        entity_id: Uuid,
        gui_state: &GuiState,
    ) {
        if let Some(scene_manager) = &gui_state.scene_manager {
            if let Some(scene) = scene_manager.get_scene(scene_id) {
                if let Some(entity) = scene.get_entity(entity_id) {
                    ui.label("Entity Details");
                    ui.separator();
                    ui.label(format!("Name: {}", entity.name));
                    ui.label(format!("ID: {}", entity_id));
                    ui.label(format!("Scene ID: {}", scene_id));
                    ui.label(format!("Number of Attributes: {}", entity.attributes.len()));
                    ui.label(format!("Number of Attached Resources: {}", entity.resource_list.len()));
                } else {
                    ui.label("Entity not found.");
                }
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

        // Get the file size
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
}
