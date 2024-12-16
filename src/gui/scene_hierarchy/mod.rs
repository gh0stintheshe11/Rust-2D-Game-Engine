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
    pub show_search: bool,
    pub popup_manager: PopupManager,
}

impl SceneHierarchy {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            show_search: false,
            popup_manager: PopupManager::new(),
        }
    }

    pub fn show(&mut self, ctx: &Context, ui: &mut Ui, gui_state: &mut GuiState) {

        // Header with integrated search
        egui::Frame {
            inner_margin: egui::Margin::ZERO,
            outer_margin: egui::Margin {
                left: 2.0,
                right: 6.0,
                top: 0.0,
                bottom: 0.0,
            },
            rounding: egui::Rounding::ZERO,
            shadow: eframe::epaint::Shadow::NONE,
            fill: egui::Color32::TRANSPARENT,
            stroke: egui::Stroke::NONE,
        }.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Scene");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Search icon/button
                    if ui.button("🔍").clicked() {
                        self.show_search = !self.show_search;
                        if !self.show_search {
                            self.search_query.clear();
                        }
                    }
                    
                    // Add button
                    if ui.button("➕").clicked() {
                        self.popup_manager.create_popup_active = true;
                    }
                    
                    // Search box (only shown when active)
                    if self.show_search {
                        ui.add(egui::TextEdit::singleline(&mut self.search_query)
                            .desired_width(150.0)
                            .hint_text("Search scenes..."));
                    }
                });
            });
            ui.separator();
        });
        

        // Display scenes in scrollable area
        egui::Frame {
            inner_margin: egui::Margin::ZERO,
            outer_margin: egui::Margin {
                left: 2.0,
                right: 2.0,
                top: 0.0,
                bottom: 0.0,
            },
            rounding: egui::Rounding::ZERO,
            shadow: eframe::epaint::Shadow::NONE,
            fill: egui::Color32::TRANSPARENT,
            stroke: egui::Stroke::NONE,
        }.show(ui, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    SceneItem::show_scenes(ui, ctx, self, gui_state);
                });
        });

        // Render popups
        self.popup_manager.render_popups(ctx, ui, gui_state);
    }
}
