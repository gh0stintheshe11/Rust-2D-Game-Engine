use crate::gui::gui_state::{GuiState, ScenePanelSelectedItem, SelectedItem};
use crate::gui::scene_hierarchy::{
    entity_item::{EntityDisplay, EntityItem},
    SceneHierarchy,
};
use egui::{Context, Ui};
use uuid::Uuid;

/// Lightweight per-frame view of a scene for the hierarchy tree.
struct SceneDisplay {
    id: Uuid,
    name: String,
    entities: Vec<EntityDisplay>,
}

pub struct SceneItem;

impl SceneItem {
    pub fn show_scenes(
        ui: &mut Ui,
        ctx: &Context,
        hierarchy: &mut SceneHierarchy,
        gui_state: &mut GuiState,
    ) {
        // Build a cheap display model (ids + names only) so the tree can
        // render while handlers freely mutate gui_state.scene_manager.
        // Cloning attribute maps every frame is what we're avoiding here.
        let mut scenes: Vec<SceneDisplay> = if let Some(scene_manager) = &gui_state.scene_manager {
            scene_manager
                .scenes
                .iter()
                .map(|(scene_id, scene)| {
                    let file_name = |path: &std::path::Path| {
                        path.file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default()
                    };
                    let mut entities: Vec<EntityDisplay> = scene
                        .entities
                        .iter()
                        .map(|(entity_id, entity)| EntityDisplay {
                            id: *entity_id,
                            name: entity.name.clone(),
                            image_names: entity.images.iter().map(|p| file_name(p)).collect(),
                            sound_names: entity.sounds.iter().map(|p| file_name(p)).collect(),
                            script_name: entity.script.as_deref().map(file_name),
                        })
                        .collect();
                    entities.sort_by_key(|e| e.name.to_lowercase());

                    SceneDisplay {
                        id: *scene_id,
                        name: scene.name.clone(),
                        entities,
                    }
                })
                .collect()
        } else {
            egui::Frame {
                inner_margin: egui::Margin {
                    left: 4,
                    right: 0,
                    top: 0,
                    bottom: 0,
                },
                outer_margin: egui::Margin::ZERO,
                corner_radius: egui::CornerRadius::ZERO,
                shadow: eframe::epaint::Shadow::NONE,
                fill: egui::Color32::TRANSPARENT,
                stroke: egui::Stroke::NONE,
            }
            .show(ui, |ui| {
                ui.label("No scenes loaded.");
            });
            return;
        };

        scenes.sort_by_key(|s| s.name.to_lowercase());

        for scene in &scenes {
            let header_id = ui.make_persistent_id(scene.id);
            egui::collapsing_header::CollapsingState::load_with_default_open(ctx, header_id, true)
                .show_header(ui, |ui| {
                    SceneItem::tree_item_scene(ui, &scene.id, &scene.name, hierarchy, gui_state);
                })
                .body(|ui| {
                    EntityItem::show_entities(
                        ui,
                        ctx,
                        hierarchy,
                        gui_state,
                        &scene.id,
                        &scene.entities,
                    );
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
        ui.horizontal(|ui| {
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
                    hierarchy
                        .popup_manager
                        .start_rename_scene(*scene_id, scene_name.to_string());
                    ui.close();
                }
                if ui.button("Delete").clicked() {
                    hierarchy.popup_manager.pending_delete =
                        Some(crate::gui::scene_hierarchy::popup::PendingDelete::Scene(
                            *scene_id,
                            scene_name.to_string(),
                        ));
                    ui.close();
                }
                if ui.button("Set Active").clicked() {
                    if let Some(scene_manager) = &mut gui_state.scene_manager {
                        let _ = scene_manager.set_active_scene(*scene_id);
                    }
                }
            });
        });
    }
}
