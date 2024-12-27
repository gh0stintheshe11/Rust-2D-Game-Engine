use eframe::egui;
use crate::gui::gui_state::GuiState;
use crate::logger::LOGGER;
use std::sync::{Arc};
use crate::project_manager::ProjectManager;

pub struct ProjectMenu;

impl ProjectMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {

        ui.add_enabled(!gui_state.project_path.as_os_str().is_empty(), egui::Button::new("Build Project")).clicked().then(|| {

            let project_path = gui_state.project_path.clone();
            let build_result = Arc::clone(&gui_state.build_result);
            let is_building = Arc::clone(&gui_state.is_building);

            {
                let mut is_building_lock = is_building.lock().unwrap();
                *is_building_lock = true;
            }

            std::thread::spawn(move || {
                let result = ProjectManager::build_project(&project_path);

                {
                    let mut result_lock = build_result.lock().unwrap();
                    *result_lock = Some(result);
                }
                {
                    let mut is_building_lock = is_building.lock().unwrap();
                    *is_building_lock = false;
                }
            });

            gui_state.show_build_project_popup = true;
        });

    }

    pub fn show_active_popup(&mut self, ctx: &egui::Context, gui_state: &mut GuiState) {
        if gui_state.show_build_project_popup {
            self.render_build_project_popup(ctx, gui_state);
        }
    }

    fn render_build_project_popup(&self, ctx: &egui::Context, gui_state: &mut GuiState) {
        let is_building = *gui_state.is_building.lock().unwrap();
        let result = gui_state.build_result.lock().unwrap().clone();

        if is_building || result.is_some() {
            // Block interactions with a transparent layer
            egui::Area::new(egui::Id::new("blocking_Layer"))
                .order(egui::Order::Foreground)
                .interactable(false)
                .show(ctx, |ui| {
                    let screen_rect = ctx.screen_rect();
                    ui.painter()
                        .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(128));
                    ui.allocate_rect(screen_rect, egui::Sense::hover());
                });

            egui::Window::new("Build Project")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    if is_building {
                        ui.vertical_centered(|ui| {
                            ui.label("Building... Please wait.");
                            ui.add(egui::Spinner::new());
                        });
                    } else if let Some(result) = result {
                        ui.vertical_centered(|ui| {
                            match &result {
                                Ok(_) => {
                                    ui.label("Build succeeded!");
                                }
                                Err(err) => {
                                    ui.label(format!("Build failed: {}", err));
                                }
                            }
                            if ui.button("OK").clicked() {
                                match result {
                                    Ok(_) => LOGGER.info("Build succeeded!"),
                                    Err(err) => LOGGER.error(format!("Build failed: {}", err)),
                                }
                                *gui_state.build_result.lock().unwrap() = None;
                                gui_state.show_build_project_popup = false;
                            }
                        });
                    }
                });

            ctx.request_repaint();
        }
    }

}
