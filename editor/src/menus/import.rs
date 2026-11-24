use crate::project::{self, AssetType};
use crate::state::GuiState;
use eframe::egui;
use rfd::FileDialog;
use rust_2d_game_engine::logger::LOGGER;

pub struct ImportMenu;

impl ImportMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        ui.add_enabled_ui(gui_state.project_open, |ui| {
            for (label, asset_type) in [
                ("Import Sound", AssetType::Sound),
                ("Import Image", AssetType::Image),
                ("Import Script", AssetType::Script),
            ] {
                if ui.button(label).clicked() {
                    ui.close();
                    Self::import_asset(gui_state, asset_type);
                }
            }
        });
    }

    fn import_asset(gui_state: &GuiState, asset_type: AssetType) {
        let Some(file_path) = FileDialog::new()
            .add_filter(format!("{:?}", asset_type), asset_type.valid_extensions())
            .pick_file()
        else {
            return;
        };
        match project::import_asset(&gui_state.project_path, &file_path, asset_type) {
            Ok(relative) => LOGGER.info(format!("Imported {}", relative)),
            Err(err) => LOGGER.error(format!("Import failed: {}", err)),
        }
    }
}
