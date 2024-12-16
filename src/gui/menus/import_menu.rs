use eframe::egui;
use crate::gui::gui_state::GuiState;
use crate::project_manager::{AssetType, ProjectManager};
use std::path::Path;
use rfd::FileDialog;

pub struct ImportMenu;

impl ImportMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        ui.add_enabled_ui(gui_state.load_project, |ui| {
            ui.button("Import Sound").clicked().then(|| {
                self.import_asset(gui_state, AssetType::Sound);
            });

            ui.button("Import Image").clicked().then(|| {
                self.import_asset(gui_state, AssetType::Image);
            });

            ui.button("Import Script").clicked().then(|| {
                self.import_asset(gui_state, AssetType::Script);
            });
        });
    }

    fn import_asset(&self, gui_state: &mut GuiState, asset_type: AssetType) {

        if let Some(file_path) = FileDialog::new()
            .add_filter(
                &format!("{:?}", asset_type),
                asset_type.valid_extensions()
            )
            .pick_file()
        {

            match ProjectManager::import_asset(
                Path::new(&gui_state.project_path),
                &file_path,
                asset_type,
            ) {
                Ok(msg) => println!("Success: {msg}"),
                Err(err) => println!("Error: {err}"),
            }
        }
    }

}
