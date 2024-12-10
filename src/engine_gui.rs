use crate::ecs::*;
use crate::project_manager::FileManagement;
use eframe::egui;

#[derive(Default)]
pub struct EngineGui {
    show_new_project_popup: bool,  // Track if the pop-up should be shown
    show_open_project_popup: bool, // Track if the pop-up should be shown
    show_new_entity_popup: bool,
    show_delete_entity_popup: bool,
    load_project: bool,      // Track if the project should be loaded
    project_name: String,    // Store the project name input
    project_path: String,    // Store the project path input
    terminal_output: String, // Store the terminal output
    entity_name: String,
    entity_id_to_delete: String,
}

impl eframe::App for EngineGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Calculate ctx width and height once at the start
        let ctx_width = ctx.screen_rect().width();

        // Display the main menu bar
        self.show_main_menu_bar(ctx, ctx_width);

        // Display the pop-up for creating a new project
        if self.show_new_project_popup {
            // Ensure the open project popup is closed
            self.show_open_project_popup = false;

            egui::Window::new("Create New Project")
                .resizable(false)
                .collapsible(false)
                .fade_in(true)
                .frame(
                    egui::Frame::window(&egui::Style::default()).shadow(egui::epaint::Shadow {
                        offset: egui::Vec2::new(0.0, 0.0), // Adjust the shadow offset
                        blur: 0.0,                         // Adjust the shadow blur
                        spread: 2.0,                       // Adjust the shadow spread
                        color: egui::Color32::DARK_GRAY,   // Add the shadow colors
                    }),
                )
                .show(ctx, |ui| {
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
                                self.project_path =
                                    format!("{}/{}", self.project_path, self.project_name);
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
                .fade_in(true)
                .frame(
                    egui::Frame::window(&egui::Style::default()).shadow(egui::epaint::Shadow {
                        offset: egui::Vec2::new(0.0, 0.0), // Adjust the shadow offset
                        blur: 0.0,                         // Adjust the shadow blur
                        spread: 2.0,                       // Adjust the shadow spread
                        color: egui::Color32::DARK_GRAY,   // Add the shadow colors
                    }),
                )
                .show(ctx, |ui| {
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
                                    let project_metadata =
                                        FileManagement::read_project_metadata(&self.project_path);
                                    self.project_name = project_metadata.project_name;
                                    self.project_path = project_metadata.project_path;
                                    // print out
                                    self.print_to_terminal(&format!(
                                        "Project name: {} in {} loaded",
                                        self.project_name, self.project_path
                                    ));
                                    self.show_open_project_popup = false; // Close the popup after opening
                                } else {
                                    self.print_to_terminal(&format!(
                                        "Invalid project path: {}",
                                        self.project_path
                                    ));
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

        // Display the pop-up for creating a new entity
        if self.show_new_entity_popup {
            egui::Window::new("Create New Entity")
                .resizable(false)
                .collapsible(false)
                .fade_in(true)
                .frame(
                    egui::Frame::window(&egui::Style::default()).shadow(egui::epaint::Shadow {
                        offset: egui::Vec2::new(0.0, 0.0),
                        blur: 0.0,
                        spread: 2.0,
                        color: egui::Color32::DARK_GRAY,
                    }),
                )
                .show(ctx, |ui| {
                    ui.label("Entity Name:");
                    ui.text_edit_singleline(&mut self.entity_name); // Reuse the project_name field or add a new field for entity name.

                    // Start horizontal layout for buttons
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            // Check if the entity name is not empty
                            if !self.project_name.is_empty() {
                                let entity_name = self.entity_name.clone();
                                let entity_path = format!("{}/entities", self.project_path);

                                // Instantiate EntityManager only here
                                let mut entity_manager = EntityManager::new(); // Assuming EntityManager has a new() function

                                // Use entity manager to handle creation
                                match entity_manager.create_entity_path(&entity_path, &entity_name)
                                {
                                    Ok(entity) => {
                                        self.print_to_terminal(&format!(
                                            "Entity '{}' created at '{}'.",
                                            entity_name, entity_path
                                        ));
                                        self.show_new_entity_popup = false; // Close the pop-up
                                    }
                                    Err(err) => {
                                        self.print_to_terminal(&format!(
                                            "Failed to create entity: {}",
                                            err
                                        ));
                                    }
                                }
                            } else {
                                self.print_to_terminal("Entity name is required.");
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_new_entity_popup = false; // Close the pop-up on cancel
                        }
                    });
                });
        }

        // Display the pop-up for deleting an entity
        if self.show_delete_entity_popup {
            egui::Window::new("Delete Entity")
                .resizable(false)
                .collapsible(false)
                .fade_in(true)
                .frame(
                    egui::Frame::window(&egui::Style::default()).shadow(egui::epaint::Shadow {
                        offset: egui::Vec2::new(0.0, 0.0),
                        blur: 0.0,
                        spread: 2.0,
                        color: egui::Color32::DARK_GRAY,
                    }),
                )
                .show(ctx, |ui| {
                    ui.label("Enter Entity ID to Delete:");
                    ui.text_edit_singleline(&mut self.entity_id_to_delete); // Input for entity ID

                    // Start horizontal layout for buttons
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            // Ensure the entity ID is not empty and is a valid number
                            if let Ok(entity_id) = self.entity_id_to_delete.parse::<usize>() {
                                let entity_path = format!("{}/entities", self.project_path);

                                // Instantiate EntityManager only here
                                let mut entity_manager = EntityManager::new();

                                // Use entity manager to handle deletion
                                match entity_manager.delete_entity_by_number(entity_id, &entity_path) {
                                    Ok(_) => {
                                        self.print_to_terminal(&format!(
                                            "Entity with ID '{}' deleted from '{}'.",
                                            entity_id, entity_path
                                        ));
                                        self.show_delete_entity_popup = false; // Close the pop-up
                                    }
                                    Err(err) => {
                                        self.print_to_terminal(&format!(
                                            "Failed to delete entity with ID {}: {}",
                                            entity_id, err
                                        ));
                                    }
                                }
                            } else {
                                self.print_to_terminal("Invalid Entity ID.");
                            }
                        }

                        if ui.button("Cancel").clicked() {
                            self.show_delete_entity_popup = false; // Close the pop-up on cancel
                        }
                    });
                });
        }

        // Display the three-panel layout
        self.show_three_panel_layout(ctx, ctx_width);
    }
}

impl EngineGui {
    // Main menu bar at the top
    fn show_main_menu_bar(&mut self, ctx: &egui::Context, ctx_width: f32) {
        egui::TopBottomPanel::top("main_menu_bar")
            .resizable(false)
            .min_height(20.0)
            .show(ctx, |ui| {
                ui.set_width(ctx_width);

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

                    // Edit menu
                    ui.menu_button("Entity", |ui| {
                        if ui.button("New").clicked() {
                            self.print_to_terminal("New Entity");
                            self.show_new_entity_popup = true; // Open the pop-up when "New" is clicked
                        }
                        if ui.button("Delete").clicked() {
                            self.print_to_terminal("Delete Entity");
                            self.show_delete_entity_popup = true;
                        }
                    });
                });
            });
    }

    // Three-panel layout with left, right, and central panels
    fn show_three_panel_layout(&mut self, ctx: &egui::Context, ctx_width: f32) {
        let item_spacing = ctx.style().spacing.item_spacing; // Get the vertical spacing between elements

        // Left panel (split into top and bottom)
        egui::SidePanel::left("asset")
            .resizable(true)
            .default_width(ctx_width * 0.15)
            .width_range((ctx_width * 0.15)..=(ctx_width * 0.3))
            .show(ctx, |ui| {
                let secondary_panel_height = ui.available_height() * 0.5;

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
                                .scroll_bar_visibility(
                                    egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                                )
                                .max_height(
                                    secondary_panel_height - heading_height - 3.0 * item_spacing.y,
                                )
                                .auto_shrink([false; 2]) // Prevent shrinking when there is less content
                                .show(ui, |ui| {
                                    let files = FileManagement::list_files_in_folder(
                                        &entity_folder_path,
                                        self,
                                    );
                                    for file in files {
                                        if ui.button(&file).clicked() {
                                            self.print_to_terminal(&format!(
                                                "Clicked on file: {}",
                                                file
                                            ));
                                        }
                                    }
                                });
                        }
                    });
            });

        // Right panel (split into top and bottom)
        egui::SidePanel::right("script")
            .resizable(true)
            .default_width(ctx_width * 0.15)
            .width_range((ctx_width * 0.15)..=(ctx_width * 0.3))
            .show(ctx, |ui| {
                let secondary_panel_height = ui.available_height() * 0.5;

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
                                .scroll_bar_visibility(
                                    egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                                )
                                .max_height(
                                    secondary_panel_height - heading_height - 3.0 * item_spacing.y,
                                )
                                .auto_shrink([false; 2]) // Prevent shrinking when there is less content
                                .show(ui, |ui| {
                                    let files = FileManagement::list_files_in_folder(
                                        &script_folder_path,
                                        self,
                                    );
                                    for file in files {
                                        if ui.button(&file).clicked() {
                                            self.print_to_terminal(&format!(
                                                "Clicked on file: {}",
                                                file
                                            ));
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
            .resizable(true)
            .min_height((ctx.screen_rect().height() - 20.0) * 0.2)
            .max_height((ctx.screen_rect().height() - 20.0) * 0.5)
            .show(ctx, |ui| {
                ui.heading("Terminal");

                // Wrap the terminal output in a scroll area
                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(
                        egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                    )
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
