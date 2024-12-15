use crate::gui::scene_hierarchy::{utils, SceneHierarchy};
use crate::gui::gui_state::{GuiState, ScenePanelSelectedItem, SelectedItem};
use egui::Ui;
use uuid::Uuid;

pub struct ResourceItem;

impl ResourceItem {
    pub fn show_resources(
        ui: &mut Ui,
        scene_id: &Uuid,
        entity_id: &Uuid,
        resource_list: &[Uuid],
        hierarchy: &mut SceneHierarchy,
        gui_state: &mut GuiState,
    ) {
        // Sort resources by name
        let mut sorted_resources: Vec<(Uuid, String)> = resource_list
            .iter()
            .filter_map(|resource_id| {
                gui_state
                    .scene_manager
                    .as_ref()
                    .and_then(|manager| manager.get_scene(*scene_id))
                    .and_then(|scene| scene.get_resource(*resource_id))
                    .map(|res| (*resource_id, res.name.clone()))
            })
            .collect();

        sorted_resources.sort_by(|(_, name_a), (_, name_b)| name_a.to_lowercase().cmp(&name_b.to_lowercase()));

        // Display sorted resources
        for (resource_id, resource_name) in sorted_resources {
            ResourceItem::tree_item_resource(
                ui,
                scene_id,
                entity_id,
                &resource_id,
                &resource_name,
                hierarchy,
                gui_state,
            );
        }
    }

    fn tree_item_resource(
        ui: &mut Ui,
        scene_id: &Uuid,
        entity_id: &Uuid,
        resource_id: &Uuid,
        resource_name: &str,
        hierarchy: &mut SceneHierarchy,
        gui_state: &mut GuiState,
    ) {
        let selected = matches!(
            gui_state.scene_panel_selected_item,
            ScenePanelSelectedItem::Resource(s_id, r_id) if s_id == *scene_id && r_id == *resource_id
        );

        let response = ui.selectable_label(selected, format!("📄 {}", resource_name));
        if response.clicked() {
            gui_state.selected_item = SelectedItem::Resource(*scene_id, *resource_id);
            gui_state.scene_panel_selected_item = ScenePanelSelectedItem::Resource(*scene_id, *resource_id);
        }

        response.context_menu(|ui| {
            if ui.button("Detach").clicked() {
                gui_state
                    .scene_manager
                    .as_mut()
                    .unwrap()
                    .get_scene_mut(*scene_id)
                    .unwrap()
                    .get_entity_mut(*entity_id)
                    .unwrap()
                    .detach_resource(*resource_id);

                utils::save_project(gui_state);

                ui.close_menu();
            }
        });
    }
}
