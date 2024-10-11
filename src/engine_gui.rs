use eframe::egui;
use crate::project_file_manager::FileManagement;

#[derive(Default)]
pub struct EngineGui {
    show_new_project_popup: bool,   // Track if the pop-up should be shown
    show_open_project_popup: bool, // Track if the pop-up should be shown
    load_project: bool,            // Track if the project should be loaded
    project_name: String,           // Store the project name input
    project_path: String,           // Store the project path input
    pub terminal_output: String,         // Store the terminal output
}

impl eframe::App for EngineGui {
    fn update(&mut self, window: &egui::Context, _frame: &mut eframe::Frame) {
        // Calculate window width and height once at the start
        let window_width = window.screen_rect().width();

        // Display the main menu bar
        self.show_main_menu_bar(window, window_width);

        // Display the pop-up for creating a new project
        if self.show_new_project_popup {
            // Ensure the open project popup is closed
            self.show_open_project_popup = false;

            egui::Window::new("Create New Project")
                .resizable(false)
                .collapsible(false)
                .frame(
                    egui::Frame::window(&egui::Style::default())
                        .shadow(egui::epaint::Shadow {
                            offset: egui::Vec2::new(0.0, 0.0), // Adjust the shadow offset
                            blur: 0.0, // Adjust the shadow blur
                            spread: 2.0, // Adjust the shadow spread
                            color: egui::Color32::DARK_GRAY, // Add the shadow colors
                        })
                )
                .show(window, |ui| {
                    ui.label("Project Name:");
                    ui.text_edit_singleline(&mut self.project_name);
        
                    ui.label("Project Path:");
                    ui.text_edit_singleline(&mut self.project_path);
        
                    // Start horizontal layout for buttons
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            // Check if the project name and path are not empty
                            if !self.project_name.is_empty() && !self.project_path.is_empty() {
                                // Store the project name and path in temporary variables
                                let project_name = self.project_name.clone();
                                let project_path = self.project_path.clone();

                                // Call FileManagement to create the project, passing self
                                FileManagement::create_project(&project_name, &project_path, self);
                                
                                // Format the project path to be a valid path
                                self.project_path = format!("{}/{}", self.project_path, self.project_name);
                                // If the project is created successfully, set the load_project to true
                                self.load_project = true;
                                self.show_new_project_popup = false; // Close the popup after creation
                            } else {
                                self.print_to_terminal("Project name and path are required.");
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_new_project_popup = false; // Close the popup on cancel
                        }
                    });
                });
        }

        if self.show_open_project_popup {
            // Ensure the new project popup is closed
            self.show_new_project_popup = false;

            egui::Window::new("Open Project")
                .resizable(false)
                .collapsible(false)
                .frame(
                    egui::Frame::window(&egui::Style::default())
                        .shadow(egui::epaint::Shadow {
                            offset: egui::Vec2::new(0.0, 0.0), // Adjust the shadow offset
                            blur: 0.0, // Adjust the shadow blur
                            spread: 2.0, // Adjust the shadow spread
                            color: egui::Color32::DARK_GRAY, // Add the shadow colors
                        })
                )
                .show(window, |ui| {
                    ui.label("Project Path:");
                    ui.text_edit_singleline(&mut self.project_path);

                    // Start horizontal layout for buttons
                    ui.horizontal(|ui| {
                        if ui.button("Open").clicked() {
                            if !self.project_path.is_empty() {
                                // Check if the path is valid and is a project
                                if FileManagement::is_valid_project_path(&self.project_path) {
                                    // Change the load_project to true
                                    self.load_project = true;
                                    // Get the project name and path from the project.json file
                                    let project_metadata = FileManagement::read_project_metadata(&self.project_path);
                                    self.project_name = project_metadata.project_name; 
                                    self.project_path = project_metadata.project_path; 
                                    // print out
                                    self.print_to_terminal(&format!("Project name: {} in {} loaded", self.project_name, self.project_path));
                                    self.show_open_project_popup = false; // Close the popup after opening
                                } else {
                                    self.print_to_terminal(&format!("Invalid project path: {}", self.project_path));
                                }
                            } else {
                                self.print_to_terminal("Project path is required.");
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_open_project_popup = false; // Close the popup on cancel
                        }
                    });
                });

        }

        // Display the three-panel layout
        self.show_three_panel_layout(window, window_width);
    }
}

impl EngineGui {

    // Main menu bar at the top
    fn show_main_menu_bar(&mut self, window: &egui::Context, window_width: f32) {

        egui::TopBottomPanel::top("main_menu_bar")
            .resizable(false)
            .min_height(20.0)
            .show(window, |ui| {
                ui.set_width(window_width);

                // Horizontal layout for File and Edit menus
                ui.horizontal(|ui| {
                    // File menu
                    ui.menu_button("File", |ui| {
                        if ui.button("New").clicked() {
                            self.show_new_project_popup = true; // Open the pop-up when "New" is clicked
                            self.show_open_project_popup = false; // Ensure open project popup is closed
                        }
                        if ui.button("Open").clicked() {
                            self.show_open_project_popup = true; // Open the pop-up when "Open" is clicked
                            self.show_new_project_popup = false; // Ensure new project popup is closed
                        }
                    });

                    // Edit menu
                    ui.menu_button("Edit", |ui| {
                        if ui.button("Undo").clicked() {
                            self.print_to_terminal("Undo");
                        }
                        if ui.button("Redo").clicked() {
                            self.print_to_terminal("Redo");
                        }
                    });
                });
            });
    }

    // Three-panel layout with left, right, and central panels
    fn show_three_panel_layout(&mut self, window: &egui::Context, window_width: f32) {

        let item_spacing = window.style().spacing.item_spacing; // Get the vertical spacing between elements

        // Left panel (split into top and bottom)
        egui::SidePanel::left("asset")
            .resizable(true)
            .default_width(window_width * 0.15)
            .width_range((window_width * 0.15)..=(window_width * 0.3))
            .show(window, |ui| {

                let secondary_panel_height = ui.available_height()*0.5;

                // Top section
                egui::TopBottomPanel::top("entity_inspector")
                    .resizable(false)
                    .exact_height(secondary_panel_height)
                    .frame(egui::Frame::none().inner_margin(egui::Margin::same(0.0)))
                    .show_inside(ui, |ui| {
                        ui.heading("Entity Inspector");
                        ui.label("Inspect and modify the attributes of the entity");
                    });
               
                // Bottom section
                let entity_folder_path = format!("{}/entities", self.project_path);
                egui::TopBottomPanel::bottom("entity")
                    .resizable(false)
                    .exact_height(secondary_panel_height)
                    .show_separator_line(false)
                    .frame(egui::Frame::none().inner_margin(egui::Margin::same(0.0)))
                    .show_inside(ui, |ui| {

                        let heading_response = ui.heading("Entities");
                        let heading_height = heading_response.rect.height(); // Get the height of the heading

                        // Wrapping the entire list of buttons in the scroll area
                        if self.load_project {
                            egui::ScrollArea::vertical()
                                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                                .max_height(secondary_panel_height - heading_height - 3.0*item_spacing.y)
                                .auto_shrink([false; 2]) // Prevent shrinking when there is less content
                                .show(ui, |ui| {
                                    let files = FileManagement::list_files_in_folder(&entity_folder_path, self);
                                    for file in files {
                                        if ui.button(&file).clicked() {
                                            self.print_to_terminal(&format!("Clicked on file: {}", file));
                                        }
                                    }
                                });
                        }
                    });

            });

        // Right panel (split into top and bottom)
        egui::SidePanel::right("script")
            .resizable(true)
            .default_width(window_width*0.15)
            .width_range((window_width*0.15)..=(window_width*0.3))
            .show(window, |ui| {

                let secondary_panel_height = ui.available_height()*0.5;

                // Top section
                egui::TopBottomPanel::top("script_inspector")
                    .resizable(false)
                    .exact_height(secondary_panel_height)
                    .frame(egui::Frame::none().inner_margin(egui::Margin::same(0.0)))
                    .show_inside(ui, |ui| {
                        ui.heading("Script Inspector");
                        ui.label("Inspect and modify the attributes of the script");
                    });
               
                // Bottom section
                let script_folder_path = format!("{}/scripts", self.project_path);
                egui::TopBottomPanel::bottom("script")
                    .resizable(false)
                    .exact_height(secondary_panel_height)
                    .show_separator_line(false)
                    .frame(egui::Frame::none().inner_margin(egui::Margin::same(0.0)))
                    .show_inside(ui, |ui| {

                        let heading_response = ui.heading("Scripts");
                        let heading_height = heading_response.rect.height(); // Get the height of the heading

                        // Wrapping the entire list of buttons in the scroll area
                        if self.load_project {
                            egui::ScrollArea::vertical()
                                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                                .max_height(secondary_panel_height - heading_height - 3.0*item_spacing.y)
                                .auto_shrink([false; 2]) // Prevent shrinking when there is less content
                                .show(ui, |ui| {
                                    let files = FileManagement::list_files_in_folder(&script_folder_path, self);
                                    for file in files {
                                        if ui.button(&file).clicked() {
                                            self.print_to_terminal(&format!("Clicked on file: {}", file));
                                        }
                                    }
                                });
                        }
                    });

            });

        // Central panel
        egui::CentralPanel::default().show(window, |ui| {
            ui.heading("Scene");
            ui.label("Scene Viewer");
        });

        // Bottom panel for terminal
        egui::TopBottomPanel::bottom("terminal")
            .resizable(true)    
            .min_height((window.screen_rect().height()-20.0)*0.2)
            .max_height((window.screen_rect().height()-20.0)*0.5)
            .show(window, |ui| {

            ui.heading("Terminal");
            
            // Wrap the terminal output in a scroll area
            egui::ScrollArea::vertical()
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                .auto_shrink([false; 2])
                .max_height(ui.available_height() - item_spacing.y)
                .stick_to_bottom(true) // Ensure the scroll area always scrolls to the bottom
                .show(ui, |ui| {
                ui.label(&self.terminal_output); // Display the terminal output
                });
            });
    }

    // Example method to add output to the terminal
    pub fn print_to_terminal(&mut self, output: &str) {
        self.terminal_output.push_str(output);
        self.terminal_output.push_str("\n"); // Add a newline for better formatting
    }
}