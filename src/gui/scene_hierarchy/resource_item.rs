use crate::gui::scene_hierarchy::SceneHierarchy;
use crate::gui::gui_state::{GuiState, ScenePanelSelectedItem, SelectedItem};
use egui::Ui;
use uuid::Uuid;
use std::path::PathBuf;
use super::utils::get_icon_for_file;
use crate::ecs::Entity;

pub struct ResourceItem;

impl ResourceItem {
    pub fn show_entity_assets(
        ui: &mut Ui,
        scene_id: Uuid,
        entity_id: Uuid,
        entity: &Entity,
        hierarchy: &mut SceneHierarchy,
        gui_state: &mut GuiState,
    ) {
        // Show Images
        for path in &entity.images {
            let selected = matches!(
                &gui_state.scene_panel_selected_item,
                ScenePanelSelectedItem::Asset(s_id, e_id, ref p)
                if *s_id == scene_id && *e_id == entity_id && p == path
            );

            let path_display = path.to_string_lossy();
            let response = ui.selectable_label(
                selected, 
                format!("{} {}", get_icon_for_file(path), path_display)
            );
            
            if response.clicked() {
                gui_state.selected_item = SelectedItem::Asset(scene_id, entity_id, path.clone());
                gui_state.scene_panel_selected_item = 
                    ScenePanelSelectedItem::Asset(scene_id, entity_id, path.clone());
            }

            let path_clone = path.clone();
            response.context_menu(|ui| {
                if ui.button("Remove").clicked() {
                    if let Some(scene_manager) = &mut gui_state.scene_manager {
                        if let Some(scene) = scene_manager.scenes.get_mut(&scene_id) {
                            if let Some(entity) = scene.get_entity_mut(entity_id) {
                                entity.images.retain(|p| p != &path_clone);
                            }
                        }
                    }
                    ui.close_menu();
                }
            });
        }

        // Show Sounds
        for path in &entity.sounds {
            let selected = matches!(
                &gui_state.scene_panel_selected_item,
                ScenePanelSelectedItem::Asset(s_id, e_id, ref p)
                if *s_id == scene_id && *e_id == entity_id && p == path
            );

            let path_display = path.to_string_lossy();
            let response = ui.selectable_label(
                selected, 
                format!("{} {}", get_icon_for_file(path), path_display)
            );
            
            if response.clicked() {
                gui_state.selected_item = SelectedItem::Asset(scene_id, entity_id, path.clone());
                gui_state.scene_panel_selected_item = 
                    ScenePanelSelectedItem::Asset(scene_id, entity_id, path.clone());
            }

            let path_clone = path.clone();
            response.context_menu(|ui| {
                if ui.button("Remove").clicked() {
                    if let Some(scene_manager) = &mut gui_state.scene_manager {
                        if let Some(scene) = scene_manager.scenes.get_mut(&scene_id) {
                            if let Some(entity) = scene.get_entity_mut(entity_id) {
                                entity.sounds.retain(|p| p != &path_clone);
                            }
                        }
                    }
                    ui.close_menu();
                }
            });
        }

        // Show Script if exists
        if let Some(script_path) = &entity.script {
            let selected = matches!(
                &gui_state.scene_panel_selected_item,
                ScenePanelSelectedItem::Asset(s_id, e_id, ref p)
                if *s_id == scene_id && *e_id == entity_id && p == script_path
            );

            let path_display = script_path.to_string_lossy();
            let response = ui.selectable_label(
                selected, 
                format!("{} {}", get_icon_for_file(script_path), path_display)
            );
            
            if response.clicked() {
                gui_state.selected_item = SelectedItem::Asset(scene_id, entity_id, script_path.clone());
                gui_state.scene_panel_selected_item = 
                    ScenePanelSelectedItem::Asset(scene_id, entity_id, script_path.clone());
            }

            response.context_menu(|ui| {
                if ui.button("Remove").clicked() {
                    if let Some(scene_manager) = &mut gui_state.scene_manager {
                        if let Some(scene) = scene_manager.scenes.get_mut(&scene_id) {
                            if let Some(entity) = scene.get_entity_mut(entity_id) {
                                entity.script = None;
                            }
                        }
                    }
                    ui.close_menu();
                }
            });
        }
    }
}
