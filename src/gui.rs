use eframe::egui;
use crate::project::FileManagement;

#[derive(Default)]
pub struct EngineGui {
    show_new_project_popup: bool,   // Track if the pop-up should be shown
    project_name: String,           // Store the project name input
    project_path: String,           // Store the project path input
}

impl eframe::App for EngineGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Calculate window width and height once at the start
        let window_width = ctx.screen_rect().width();
        let window_height = ctx.screen_rect().height();

        // Display the main menu bar
        self.show_main_menu_bar(ctx, window_width);

        // Display the pop-up for creating a new project (without shade)
        if self.show_new_project_popup {
            egui::Window::new("Create New Project")
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("Project Name:");
                    ui.text_edit_singleline(&mut self.project_name);
        
                    ui.label("Project Path:");
                    ui.text_edit_singleline(&mut self.project_path);
        
                    // Start horizontal layout for buttons
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            if !self.project_name.is_empty() && !self.project_path.is_empty() {
                                // Call FileManagement to create the project
                                FileManagement::create_project(&self.project_name, &self.project_path);
                                self.show_new_project_popup = false; // Close the popup after creation
                            } else {
                                ui.label("Both fields are required.");
                            }
                        }
        
                        if ui.button("Cancel").clicked() {
                            self.show_new_project_popup = false; // Close the popup on cancel
                        }
                    });
                });
        }

        // Display the three-panel layout
        self.show_three_panel_layout(ctx, window_width, window_height);
    }
}

impl EngineGui {
    // Main menu bar at the top
    fn show_main_menu_bar(&mut self, ctx: &egui::Context, window_width: f32) {

        egui::TopBottomPanel::top("main_menu_bar")
            .resizable(false)
            .min_height(20.0)
            .show(ctx, |ui| {
                ui.set_width(window_width);

                // Horizontal layout for File and Edit menus
                ui.horizontal(|ui| {
                    // File menu
                    ui.menu_button("File", |ui| {
                        if ui.button("New").clicked() {
                            self.show_new_project_popup = true; // Open the pop-up when "New" is clicked
                        }
                        if ui.button("Open").clicked() {
                            println!("Open file");
                        }
                    });

                    // Edit menu
                    ui.menu_button("Edit", |ui| {
                        if ui.button("Undo").clicked() {
                            println!("Undo");
                        }
                        if ui.button("Redo").clicked() {
                            println!("Redo");
                        }
                    });
                });
            });
    }

    // Three-panel layout with left, right, and central panels
    fn show_three_panel_layout(&self, ctx: &egui::Context, window_width: f32, window_height: f32) {
        // Left panel (split into top and bottom)
        egui::SidePanel::left("asset")
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_width(window_width * 0.15);

                // Split the left panel into top and bottom using TopBottomPanel
                egui::TopBottomPanel::top("asset_inspector")
                    .resizable(false)
                    .min_height((window_height - 20.0) * 0.5)
                    .show_inside(ui, |ui| {
                        ui.heading("Asset Inspector");
                        ui.label("Inspect and modify the attributes of the asset");
                    });

                egui::TopBottomPanel::bottom("asset_browser")
                    .resizable(false)
                    .min_height((window_height - 20.0) * 0.5)
                    .show_inside(ui, |ui| {
                        ui.heading("Asset Browser");
                        ui.label("Browse and select the asset");
                    });
            });

        // Right panel (split into top and bottom)
        egui::SidePanel::right("script")
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_width(window_width * 0.15);

                // Split the right panel into top and bottom using TopBottomPanel
                egui::TopBottomPanel::top("script_inspector")
                    .resizable(false)
                    .min_height(window_height * 0.5)
                    .show_inside(ui, |ui| {
                        ui.heading("Script Inspector");
                        ui.label("Inspect and modify the attributes of the script");
                    });

                egui::TopBottomPanel::bottom("script_browser")
                    .resizable(false)
                    .min_height(window_height * 0.5)
                    .show_inside(ui, |ui| {
                        ui.heading("Script Browser");
                        ui.label("Browse and select the script");
                    });
            });

        // Central panel
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Scene");
            ui.label("Scene Viewer");
        });

        // Bottom panel for terminal
        egui::TopBottomPanel::bottom("terminal")
            .resizable(false)
            .min_height(window_height * 0.15)
            .show(ctx, |ui| {
                let bottom_panel_width = ui.available_width();
                ui.set_width(bottom_panel_width);
                ui.heading("Terminal");
                ui.label("Display the system output of the engine");
            });
    }
}