use crate::menus::{
    edit::EditMenu, file::FileMenu, import::ImportMenu, project::ProjectMenu, view::ViewMenu,
};
use crate::state::GuiState;
use eframe::egui;

pub struct MenuBar {
    pub file_menu: FileMenu,
    pub edit_menu: EditMenu,
    pub view_menu: ViewMenu,
    pub import_menu: ImportMenu,
    pub project_menu: ProjectMenu,
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}

impl MenuBar {
    pub fn new() -> Self {
        Self {
            file_menu: FileMenu::new(),
            edit_menu: EditMenu::new(),
            view_menu: ViewMenu::new(),
            import_menu: ImportMenu::new(),
            project_menu: ProjectMenu::new(),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, gui_state: &mut GuiState) {
        ui.horizontal(|ui| {
            // File menu
            ui.menu_button("File", |ui| {
                self.file_menu.show(ctx, ui, gui_state);
            });

            // Render popup window globally
            self.file_menu.show_active_popup(ctx, gui_state);

            // Edit menu
            ui.menu_button("Edit", |ui| {
                self.edit_menu.show(ctx, ui, gui_state);
            });

            // View menu
            ui.menu_button("View", |ui| {
                self.view_menu.show(ctx, ui, gui_state);
            });

            // Import menu
            ui.menu_button("Import", |ui| {
                self.import_menu.show(ctx, ui, gui_state);
            });

            // Project menu
            ui.menu_button("Project", |ui| {
                self.project_menu.show(ctx, ui, gui_state);
            });

            // Render popup window globally
            self.project_menu.show_active_popup(ctx, gui_state);
        });
    }
}
