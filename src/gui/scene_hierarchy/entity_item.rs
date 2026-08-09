use crate::gui::gui_state::{GuiState, ScenePanelSelectedItem, SelectedItem};
use crate::gui::scene_hierarchy::SceneHierarchy;
use egui::{Context, Ui};
use uuid::Uuid;

/// Lightweight per-frame view of an entity for the hierarchy tree.
/// Only names are copied - never the attribute map.
pub struct EntityDisplay {
    pub id: Uuid,
    pub name: String,
    pub image_names: Vec<String>,
    pub sound_names: Vec<String>,
    pub script_name: Option<String>,
    pub script_path: Option<std::path::PathBuf>,
}

pub struct EntityItem;

impl EntityItem {
    pub fn show_entities(
        ui: &mut Ui,
        ctx: &Context,
        hierarchy: &mut SceneHierarchy,
        gui_state: &mut GuiState,
        scene_id: &Uuid,
        entities: &[EntityDisplay],
    ) {
        for entity in entities {
            if !hierarchy.search_query.is_empty()
                && !entity
                    .name
                    .to_lowercase()
                    .contains(&hierarchy.search_query.to_lowercase())
            {
                continue;
            }

            let header_id = ui.make_persistent_id(entity.id);

            // Show as collapsable if it has any attached assets
            let has_assets = !entity.image_names.is_empty()
                || !entity.sound_names.is_empty()
                || entity.script_name.is_some();

            if has_assets {
                egui::collapsing_header::CollapsingState::load_with_default_open(
                    ctx, header_id, true,
                )
                .show_header(ui, |ui| {
                    EntityItem::tree_item_entity(
                        ui,
                        scene_id,
                        &entity.id,
                        &entity.name,
                        hierarchy,
                        gui_state,
                    );
                })
                .body(|ui| {
                    for filename in &entity.image_names {
                        ui.horizontal(|ui| {
                            ui.label(format!("🔆 {}", filename));
                        });
                    }
                    for filename in &entity.sound_names {
                        ui.horizontal(|ui| {
                            ui.label(format!("🎵 {}", filename));
                        });
                    }
                    if let Some(filename) = &entity.script_name {
                        ui.horizontal(|ui| {
                            // Clicking a script opens it in the code editor
                            if ui
                                .selectable_label(false, format!("📄 {}", filename))
                                .on_hover_text("Open in the script editor")
                                .clicked()
                            {
                                gui_state.open_script_request = entity.script_path.clone();
                            }
                        });
                    }
                });
            } else {
                ui.horizontal(|ui| {
                    EntityItem::tree_item_entity(
                        ui,
                        scene_id,
                        &entity.id,
                        &entity.name,
                        hierarchy,
                        gui_state,
                    );
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
        let display_name = if let Some(name) = entity_name.split('/').next_back() {
            name
        } else {
            entity_name
        };

        let response = ui.selectable_label(selected, format!("🖼 {}", display_name));
        if response.clicked() {
            gui_state.selected_item = SelectedItem::Entity(*scene_id, *entity_id);
            gui_state.scene_panel_selected_item =
                ScenePanelSelectedItem::Entity(*scene_id, *entity_id);
        }

        response.context_menu(|ui| {
            if ui.button("Attach Asset").clicked() {
                hierarchy.popup_manager.resource_selection = Some((*scene_id, *entity_id));
                hierarchy.popup_manager.resource_selection_popup_active = true;
                ui.close();
            }
            if ui.button("Detach Asset").clicked() {
                hierarchy.popup_manager.manage_assets_entity = Some((*scene_id, *entity_id));
                hierarchy.popup_manager.manage_assets_popup_active = true;
                ui.close();
            }
            if ui.button("Rename").clicked() {
                hierarchy.popup_manager.entity_rename_entity = Some((*scene_id, *entity_id));
                hierarchy.popup_manager.rename_input = entity_name.to_string();
                ui.close();
            }
            if ui.button("Delete").clicked() {
                hierarchy.popup_manager.pending_delete =
                    Some(crate::gui::scene_hierarchy::popup::PendingDelete::Entity(
                        *scene_id,
                        *entity_id,
                        entity_name.to_string(),
                    ));
                ui.close();
            }
        });
    }
}
