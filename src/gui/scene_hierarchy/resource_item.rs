use crate::gui::scene_hierarchy::{utils, SceneHierarchy};
use crate::gui::gui_state::{GuiState, ScenePanelSelectedItem, SelectedItem};
use crate::ecs::Resource;
use egui::Ui;
use uuid::Uuid;

pub struct ResourceItem;

impl ResourceItem {
    pub fn show_resources(
        ui: &mut Ui,
        scene_id: &Uuid,
        _entity_id: &Uuid,
        resource_list: &Vec<Uuid>,
        hierarchy: &mut SceneHierarchy,
        gui_state: &mut GuiState,
    ) {
        // First collect all the data we need
        let resources_data: Vec<(Uuid, String)> = if let Some(scene_manager) = &gui_state.scene_manager {
            if let Some(scene) = scene_manager.scenes.get(scene_id) {
                resource_list
                    .iter()
                    .filter_map(|&resource_id| {
                        scene.resources.get(&resource_id)
                            .map(|resource| (resource_id, resource.name.clone()))
                    })
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Then show the resources using the collected data
        for (resource_id, name) in resources_data {
            let selected = matches!(
                gui_state.scene_panel_selected_item,
                ScenePanelSelectedItem::Resource(s_id, r_id) if s_id == *scene_id && r_id == resource_id
            );

            let response = ui.selectable_label(selected, format!("📄 {}", name));
            
            if response.clicked() {
                gui_state.selected_item = SelectedItem::Resource(*scene_id, resource_id);
                gui_state.scene_panel_selected_item = ScenePanelSelectedItem::Resource(*scene_id, resource_id);
            }

            response.context_menu(|ui| {
                if ui.button("Delete").clicked() {
                    if let Some(scene_manager) = &mut gui_state.scene_manager {
                        if let Some(scene) = scene_manager.scenes.get_mut(scene_id) {
                            scene.resources.remove(&resource_id);
                        }
                    }
                    ui.close_menu();
                }
            });
        }
    }
}
