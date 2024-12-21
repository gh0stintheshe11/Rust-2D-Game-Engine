use crate::gui::scene_hierarchy::{SceneHierarchy, resource_item::ResourceItem};
use crate::gui::gui_state::{GuiState, ScenePanelSelectedItem, SelectedItem};
use egui::{Context, Ui};
use uuid::Uuid;
use indexmap::IndexMap;

pub struct EntityItem;

impl EntityItem {
    pub fn show_entities(
        ui: &mut Ui,
        ctx: &Context,
        hierarchy: &mut SceneHierarchy,
        gui_state: &mut GuiState,
        scene_id: &Uuid,
        entities: &IndexMap<Uuid, crate::ecs::Entity>,
    ) {
        let mut sorted_entities: Vec<(&Uuid, &crate::ecs::Entity)> = entities.iter().collect();
        sorted_entities.sort_by(|(_, entity_a), (_, entity_b)| {
            entity_a.name.to_lowercase().cmp(&entity_b.name.to_lowercase())
        });

        for (entity_id, entity) in sorted_entities {
            if !hierarchy.search_query.is_empty()
                && !entity.name.to_lowercase().contains(&hierarchy.search_query.to_lowercase()) {
                continue;
            }

            let header_id = ui.make_persistent_id(entity_id);

            // Show as collapsable if has images or sounds, otherwise show as label
            if !entity.images.is_empty() || !entity.sounds.is_empty() {
                egui::collapsing_header::CollapsingState::load_with_default_open(ctx, header_id, true)
                    .show_header(ui, |ui| {
                        EntityItem::tree_item_entity(ui, scene_id, entity_id, &entity.name, hierarchy, gui_state);
                    })
                    .body(|ui| {
                        if !entity.images.is_empty() {
                            for path in &entity.images {
                                let filename = path.file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string();
                                ui.horizontal(|ui| {
                                    ui.label(format!("🔆 {}", filename));
                                });
                            }
                        }
                        if !entity.sounds.is_empty() {
                            for path in &entity.sounds {
                                let filename = path.file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string();
                                ui.horizontal(|ui| {
                                    ui.label(format!("🎵 {}", filename));
                                });
                            }
                        }
                        if entity.script.is_some() {
                                let filename = entity.script.clone().unwrap().file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string();
                                ui.horizontal(|ui| {
                                    ui.label(format!("🎵 {}", filename));
                                });
                        }
                    });
            } else {
                ui.horizontal(|ui| {
                    EntityItem::tree_item_entity(ui, scene_id, entity_id, &entity.name, hierarchy, gui_state);
                });
            }
        }
    }

    pub fn tree_item_entity(
        ui: &mut Ui,
        scene_id: &Uuid,
        entity_id: &Uuid,
        entity_name: &str,
        hierarchy: &mut SceneHierarchy,
        gui_state: &mut GuiState,
    ) {
        let selected = matches!(
            gui_state.scene_panel_selected_item,
            ScenePanelSelectedItem::Entity(s_id, e_id) if s_id == *scene_id && e_id == *entity_id
        );

        // Just show the filename without path
        let display_name = if let Some(name) = entity_name.split('/').last() {
            name
        } else {
            entity_name
        };

        let response = ui.selectable_label(selected, format!("🖼 {}", display_name));
        if response.clicked() {
            gui_state.selected_item = SelectedItem::Entity(*scene_id, *entity_id);
            gui_state.scene_panel_selected_item = ScenePanelSelectedItem::Entity(*scene_id, *entity_id);
        }

        response.context_menu(|ui| {
            if ui.button("Attach Asset").clicked() {
                hierarchy.popup_manager.resource_selection = Some((*scene_id, *entity_id));
                hierarchy.popup_manager.resource_selection_popup_active = true;
                ui.close_menu();
            }
            if ui.button("Detach Asset").clicked() {
                hierarchy.popup_manager.manage_assets_entity = Some((*scene_id, *entity_id));
                hierarchy.popup_manager.manage_assets_popup_active = true;
                ui.close_menu();
            }
            if ui.button("Rename").clicked() {
                hierarchy.popup_manager.entity_rename_entity = Some((*scene_id, *entity_id));
                hierarchy.popup_manager.rename_input = entity_name.to_string();
                ui.close_menu();
            }
            if ui.button("Delete").clicked() {
                gui_state
                    .scene_manager
                    .as_mut()
                    .unwrap()
                    .get_scene_mut(*scene_id)
                    .unwrap()
                    .delete_entity(*entity_id);
                ui.close_menu();
            }
        });
    }
}
