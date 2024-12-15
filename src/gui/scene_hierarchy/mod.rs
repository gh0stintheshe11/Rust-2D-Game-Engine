pub mod entity_item;
pub mod popup;
pub mod predefined_entities;
pub mod resource_item;
pub mod scene_item;
pub mod utils;

use crate::gui::gui_state::GuiState;
use egui::{Context, Ui};
use popup::PopupManager;
use scene_item::SceneItem;

pub struct SceneHierarchy {
    pub search_query: String,
    pub popup_manager: PopupManager,
}

impl SceneHierarchy {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            popup_manager: PopupManager::new(),
        }
    }

    pub fn show(&mut self, ctx: &Context, ui: &mut Ui, gui_state: &mut GuiState) {

        // Menu bar
        ui.horizontal(|ui| {
            if ui.button("+").clicked() {
                self.popup_manager.create_popup_active = true;
            }
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.search_query);
        });

        ui.separator();

        // Display scenes
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                {
                    SceneItem::show_scenes(ui, ctx, self, gui_state);
                }
            });

        // Render popups
        self.popup_manager.render_popups(ctx, ui, gui_state);

    }
}
