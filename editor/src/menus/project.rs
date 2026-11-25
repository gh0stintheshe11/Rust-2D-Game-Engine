use crate::panels::input_actions::InputActionsWindow;
use crate::state::GuiState;
use eframe::egui;
use rust_2d_game_engine::logger::LOGGER;
use std::sync::{Arc, Mutex};

pub struct ProjectMenu {
    pub input_actions: InputActionsWindow,
}

impl ProjectMenu {
    pub fn new() -> Self {
        Self {
            input_actions: InputActionsWindow::default(),
        }
    }

    pub fn show(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        if ui
            .add_enabled(gui_state.project_open, egui::Button::new("Input Actions…"))
            .on_hover_text("Named controls such as \"jump\", and the keys and buttons for each")
            .clicked()
        {
            self.input_actions.open = true;
            ui.close();
        }
        let enabled = gui_state.project_open && !*gui_state.is_building.lock().unwrap();
        if ui
            .add_enabled(enabled, egui::Button::new("Build Project"))
            .on_hover_text("cargo build --release; the game ends up in target/release")
            .clicked()
        {
            // The shell saves the script editor buffer, then calls start_build
            gui_state.build_requested = true;
            ui.close();
        }
        if ui
            .add_enabled(gui_state.project_open, egui::Button::new("Clear Save Data"))
            .on_hover_text(
                "Delete what the game saved with save_data while playing in the editor \
                 (saves/save.json)",
            )
            .clicked()
        {
            gui_state.clear_save_data_requested = true;
            ui.close();
        }
    }

    pub fn show_active_popup(&mut self, ctx: &egui::Context, gui_state: &mut GuiState) {
        self.input_actions.show(ctx, gui_state);
        if gui_state.show_build_project_popup {
            Self::render_build_project_popup(ctx, gui_state);
        }
    }

    fn render_build_project_popup(ctx: &egui::Context, gui_state: &mut GuiState) {
        let is_building = *gui_state.is_building.lock().unwrap();
        let result = gui_state.build_result.lock().unwrap().clone();

        // A modal blocks the rest of the editor while it is open
        egui::Modal::new(egui::Id::unique("build_project_modal")).show(ctx, |ui| {
            ui.set_min_width(260.0);
            ui.vertical_centered(|ui| {
                ui.heading("Build Project");
                ui.add_space(8.0);
                if is_building {
                    ui.label("Building… see the console's Debug tab for cargo output.");
                    ui.add(egui::Spinner::new());
                } else {
                    match &result {
                        Some(Ok(())) => ui.label("Build succeeded. The game is in target/release."),
                        Some(Err(err)) => ui.colored_label(egui::Color32::RED, err),
                        None => {
                            ui.colored_label(egui::Color32::RED, "The build stopped unexpectedly.")
                        }
                    };
                    ui.add_space(8.0);
                    if ui.button("OK").clicked() {
                        *gui_state.build_result.lock().unwrap() = None;
                        gui_state.show_build_project_popup = false;
                    }
                }
            });
        });
        if is_building {
            ctx.request_repaint();
        }
    }
}

/// Build the open project on a background thread and show the progress
/// popup. Call after saving everything the build should include.
pub fn start_build(gui_state: &mut GuiState) {
    let project_path = gui_state.project_path.clone();
    let build_result = Arc::clone(&gui_state.build_result);
    let is_building = Arc::clone(&gui_state.is_building);
    *is_building.lock().unwrap() = true;
    gui_state.show_build_project_popup = true;
    LOGGER.info(format!("Building {}", project_path.display()));

    std::thread::spawn(move || {
        // Clears the flag even if the build panics, so the popup can close
        let _building = BuildingFlag(Arc::clone(&is_building));
        let result = crate::project::build_project(&project_path);
        match &result {
            Ok(()) => LOGGER.info("Build succeeded"),
            Err(err) => LOGGER.error(format!("Build failed: {}", err)),
        }
        *build_result.lock().unwrap_or_else(|e| e.into_inner()) = Some(result);
    });
}

/// Resets the "building" flag when dropped.
struct BuildingFlag(Arc<Mutex<bool>>);

impl Drop for BuildingFlag {
    fn drop(&mut self) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = false;
    }
}
