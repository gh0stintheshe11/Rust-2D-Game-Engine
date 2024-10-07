use eframe::egui;
use crate::project::FileManagement;

#[derive(Default)]
pub struct EngineGui {
    show_new_project_popup: bool,   // Track if the pop-up should be shown
    show_open_project_popup: bool, // Track if the pop-up should be shown
    load_project: bool,            // Track if the project should be loaded
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

        if self.show_open_project_popup {
            egui::Window::new("Open Project")
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("Project Path:");
                    ui.text_edit_singleline(&mut self.project_path);

                    // Start horizontal layout for buttons
                    ui.horizontal(|ui| {
                        if ui.button("Open").clicked() {
                            if !self.project_path.is_empty() {
                                //check if the path is valid and is a project
                                if FileManagement::is_valid_project_path(&self.project_path) {
                                    //change the load_project to true
                                    self.load_project = true;
                                    self.show_open_project_popup = false; // Close the popup after creation
                                } else {
                                    ui.label("Invalid project path.");
                                }
                            } else {
                                ui.label("Both fields are required.");
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_open_project_popup = false; // Close the popup on cancel
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
                            self.show_open_project_popup = true; // Open the pop-up when "Open" is clicked
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
            .min_width(window_width*0.15)
            .max_width(window_width*0.15)
            .show(ctx, |ui| {
                
                // Split the left panel into top and bottom using TopBottomPanel
                egui::TopBottomPanel::top("entity_inspector")
                    .resizable(false)
                    .min_height((window_height - 20.0) * 0.5)
                    .max_height((window_height - 20.0) * 0.5)
                    .show_inside(ui, |ui| {
                        ui.heading("Entity Inspector");
                        ui.label("Inspect and modify the attributes of the entity");
                    });

                
                // Load the project_path + ./entities
                let entity_folder_path = format!("{}/entities", self.project_path);
                egui::TopBottomPanel::bottom("entity_browser")
                    .resizable(false)
                    .min_height((window_height - 20.0) * 0.5)
                    .max_height((window_height - 20.0) * 0.5)
                    .show_inside(ui, |ui| {
                        ui.heading("Entities");
                        // Wrapping the entire list of buttons in the scroll area
                        if self.load_project {
                        egui::ScrollArea::vertical()
                            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                            .max_height((window_height - 100.0) * 0.5)
                            .max_width(window_width*0.15)
                            .auto_shrink([false; 2]) // Prevent shrinking when there is less content
                            .show(ui, |ui| {
                                let files = FileManagement::list_files_in_folder(&entity_folder_path);
                                for file in files {
                                    if ui.button(&file).clicked() {
                                        println!("Clicked on file: {}", file);
                                    }
                                }
                            });
                        }
                    });
            });

        // Right panel (split into top and bottom)
        egui::SidePanel::right("script")
            .resizable(false)
            .min_width(window_width*0.15)
            .max_width(window_width*0.15)
            .show(ctx, |ui| {

                // Split the right panel into top and bottom using TopBottomPanel
                egui::TopBottomPanel::top("script_inspector")
                    .resizable(false)
                    .frame(egui::Frame::none()) // Set to no frame
                    .min_height((window_height-20.0) * 0.5)
                    .show_inside(ui, |ui| {
                        ui.heading("Script Inspector");
                        ui.label("Inspect and modify the attributes of the script");
                    });


                // Load the project_path + ./scripts
                let entity_folder_path = format!("{}/scripts", self.project_path);
                egui::TopBottomPanel::bottom("script_browser")
                    .resizable(false)
                    .min_height((window_height - 20.0) * 0.5)
                    .max_height((window_height - 20.0) * 0.5)
                    .show_inside(ui, |ui| {
                        ui.heading("Scripts");
                        // Wrapping the entire list of buttons in the scroll area
                        if self.load_project {
                        egui::ScrollArea::vertical()
                            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                            .max_height((window_height - 100.0) * 0.5)
                            .max_width(window_width*0.15)
                            .auto_shrink([false; 2]) // Prevent shrinking when there is less content
                            .show(ui, |ui| {
                                let files = FileManagement::list_files_in_folder(&entity_folder_path);
                                for file in files {
                                    if ui.button(&file).clicked() {
                                        println!("Clicked on file: {}", file);
                                    }
                                }
                            });
                        }
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