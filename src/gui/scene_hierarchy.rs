use eframe::egui;

pub struct SceneHierarchy {
}

impl SceneHierarchy {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&self, ui: &mut egui::Ui) {
        ui.label("Scene tree will go here");
    }
}