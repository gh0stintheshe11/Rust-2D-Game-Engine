use crate::gui::scene_hierarchy::{SceneHierarchy, entity_item::EntityItem};
use crate::gui::gui_state::{GuiState, SelectedItem, ScenePanelSelectedItem};
use egui::{Context, Ui};
use uuid::Uuid;
use crate::ecs::Scene;

pub struct SceneItem;

impl SceneItem {
    pub fn show_scenes(
        ui: &mut Ui,
        ctx: &Context,
        hierarchy: &mut SceneHierarchy,
        gui_state: &mut GuiState,
    ) {
        let scenes = if let Some(scene_manager) = &gui_state.scene_manager {
            scene_manager.scenes.clone()
        } else {
            egui::Frame {
                inner_margin: egui::Margin { left: 4.0, right: 0.0, top: 0.0, bottom: 0.0 },
                outer_margin: egui::Margin::ZERO,
                rounding: egui::Rounding::ZERO,
                shadow: eframe::epaint::Shadow::NONE,
                fill: egui::Color32::TRANSPARENT,
                stroke: egui::Stroke::NONE,
            }.show(ui, |ui| {
                ui.label("No scenes loaded.");
            });
            return;
        };

        let mut sorted_scenes: Vec<(&Uuid, &Scene)> = scenes.iter().collect();
        sorted_scenes.sort_by(|(_, scene_a), (_, scene_b)| {
            scene_a.name.to_lowercase().cmp(&scene_b.name.to_lowercase())
        });

        for (scene_id, scene) in sorted_scenes {
            let header_id = ui.make_persistent_id(scene_id);
            egui::collapsing_header::CollapsingState::load_with_default_open(ctx, header_id, true)
                .show_header(ui, |ui| {
                    SceneItem::tree_item_scene(ui, scene_id, &scene.name, hierarchy, gui_state);
                })
                .body(|ui| {
                    EntityItem::show_entities(ui, ctx, hierarchy, gui_state, scene_id, &scene.entities);
                });
        }
    }

    fn tree_item_scene(
        ui: &mut Ui,
        scene_id: &Uuid,
        scene_name: &str,
        hierarchy: &mut SceneHierarchy,
        gui_state: &mut GuiState,
    ) {
        let selected = matches!(
            gui_state.scene_panel_selected_item,
            ScenePanelSelectedItem::Scene(s_id) if s_id == *scene_id
        );

        let response = ui.selectable_label(selected, scene_name);
        if response.clicked() {
            gui_state.selected_item = SelectedItem::Scene(*scene_id);
            gui_state.scene_panel_selected_item = ScenePanelSelectedItem::Scene(*scene_id);
        }

        response.context_menu(|ui| {
            if ui.button("Rename").clicked() {
                hierarchy.popup_manager.start_rename_scene(*scene_id, scene_name.to_string());
                ui.close_menu();
            }
            if ui.button("Delete").clicked() {
                gui_state.scene_manager.as_mut().unwrap().delete_scene(*scene_id);
                ui.close_menu();
            }
        });
    }
}
