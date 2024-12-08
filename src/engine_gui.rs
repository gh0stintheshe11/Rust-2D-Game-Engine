use crate::audio_engine::AudioEngine;
use crate::ecs::AttributeValueType;
use crate::ecs::Entity;
use crate::ecs::EntityManager;
use rfd::FileDialog;


use crate::physics_engine::PhysicsEngine;
use crate::project_manager::FileManagement;
use crate::render_engine::RenderEngine;
use eframe::egui;
use eframe::glow::SET;

// #[derive(Default)]
pub struct EngineGui {
    ecs: EntityManager,
    render_engine: RenderEngine,
    physics_engine: PhysicsEngine,
    audio_engine: AudioEngine,
    show_new_project_popup: bool,  // Track if the pop-up should be shown
    show_open_project_popup: bool, // Track if the pop-up should be shown
    load_project: bool,            // Track if the project should be loaded
    project_name: String,          // Store the project name input
    project_path: String,          // Store the project path input
    terminal_output: String,       // Store the terminal output
    // entities panel
    highlighted_entity: Option<usize>,
    // entity inspector panel
    show_add_attribute_popup: bool,
    selected_attribute_type: AttributeValueType,
    new_attribute_name: String,
    new_attribute_value: String,
    // scripts panel
    highlighted_script: Option<usize>,
    next_script_id: usize,
    current_script_content: String,
}

impl Default for EngineGui {
    fn default() -> Self {
        Self {
            ecs: EntityManager::new(),
            render_engine: RenderEngine::new(),
            physics_engine: PhysicsEngine::new(),
            audio_engine: AudioEngine::new(),
            show_new_project_popup: false,
            show_open_project_popup: false,
            load_project: false,
            project_name: String::new(),
            project_path: String::new(),
            terminal_output: String::new(),
            // entities panel
            highlighted_entity: None,
            // entity inspector panel
            show_add_attribute_popup: false,
            selected_attribute_type: AttributeValueType::String(String::new()),
            new_attribute_name: String::new(),
            new_attribute_value: String::new(),
            // scripts panel
            highlighted_script: None,
            next_script_id: 0,
            current_script_content: String::new(),
        }
    }
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
                        self.show_import_menu(ui); // Add "Import..." submenu
                    });

                    // Edit menu
                    ui.menu_button("Edit", |ui| {
                        if ui.button("Undo").clicked() {
                            self.print_to_terminal("Undo");
                        }
                        if ui.button("Redo").clicked() {
                            self.print_to_terminal("Redo");
                        }

                        // Add submenu "Add..." inside edit menu
                        ui.menu_button("Add...", |ui| {
                            if ui.button("new scene").clicked() {
                                self.print_to_terminal("New scene added.");
                            }
                            if ui.button("new entity").clicked() {
                                if self.load_project {
                                    self.add_entity();
                                    self.print_to_terminal("New entity added.");
                                }
                            }
                            if ui.button("new script").clicked() {
                                if self.load_project {
                                    self.add_script();
                                    self.print_to_terminal("New script added.");
                                }
                            }
                        });
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

                        if self.highlighted_entity.is_some() {
                            // Add Attribute Button
                            if ui.button("Add Attribute").clicked() {
                                self.show_add_attribute_popup = true;
                            }
                        }

                        if let Some(selected_id) = self.highlighted_entity {

                            ui.label(format!("Entity ID: {}", selected_id));
                            ui.label("Attributes:");

                            // Display attributes of the Entity
                            if let Some(attributes) =
                                self.ecs.get_attributes_by_entity_id(selected_id)
                            {
                                for (key, attribute) in attributes {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{}: {:?}", key, attribute.value_type));
                                        if ui.button("Delete").clicked() {
                                            self.ecs
                                                .delete_attribute_by_entity_id(selected_id, &key);
                                            self.update_entity(selected_id);
                                            self.print_to_terminal(&format!(
                                                "Deleted attribute '{}'.",
                                                key
                                            ));
                                        }
                                    });
                                }
                            } else {
                                ui.label("Selected entity not found.");
                            }
                        } else {
                            ui.label("Inspect and modify the attributes of the entity");
                        }

                        // Add Attribute Popup
                        if self.show_add_attribute_popup {
                            egui::Window::new("Add Attribute")
                                .resizable(false)
                                .collapsible(false)
                                .show(ctx, |ui| {
                                    ui.label("Attribute Name:");
                                    ui.text_edit_singleline(&mut self.new_attribute_name);

                                    ui.label("Attribute Type:");
                                    egui::ComboBox::from_label("Type")
                                        .selected_text(format!(
                                            "{:?}",
                                            self.selected_attribute_type
                                        ))
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                &mut self.selected_attribute_type,
                                                AttributeValueType::Integer(0),
                                                "Integer",
                                            );
                                            ui.selectable_value(
                                                &mut self.selected_attribute_type,
                                                AttributeValueType::Float(0.0),
                                                "Float",
                                            );
                                            ui.selectable_value(
                                                &mut self.selected_attribute_type,
                                                AttributeValueType::String(String::new()),
                                                "String",
                                            );
                                            ui.selectable_value(
                                                &mut self.selected_attribute_type,
                                                AttributeValueType::Boolean(false),
                                                "Boolean",
                                            );
                                        });

                                    ui.label("Value:");
                                    ui.text_edit_singleline(&mut self.new_attribute_value);

                                    ui.horizontal(|ui| {
                                        if ui.button("OK").clicked() {
                                            if let Some(selected_id) = self.highlighted_entity {
                                                let attribute_name =
                                                    self.new_attribute_name.trim().to_string();
                                                let attribute_value =
                                                    self.new_attribute_value.trim().to_string();

                                                if attribute_name.is_empty() {
                                                    self.print_to_terminal(
                                                        "Attribute name cannot be empty.",
                                                    );
                                                    return;
                                                }

                                                match self.ecs.add_attribute_by_entity_id(
                                                    selected_id,
                                                    attribute_name.clone(),
                                                    self.selected_attribute_type.clone(),
                                                    attribute_value.clone(),
                                                ) {
                                                    Ok(_) => {
                                                        self.update_entity(selected_id);
                                                        self.print_to_terminal(
                                                            "Attribute added successfully.",
                                                        );
                                                        self.new_attribute_name.clear();
                                                        self.new_attribute_value.clear();
                                                        self.show_add_attribute_popup = false;
                                                    }
                                                    Err(err) => {
                                                        self.print_to_terminal(&format!(
                                                            "Error adding attribute: {}",
                                                            err
                                                        ));
                                                    }
                                                }
                                            } else {
                                                self.print_to_terminal("No entity selected.");
                                            }
                                        }

                                        if ui.button("Cancel").clicked() {
                                            self.show_add_attribute_popup = false;
                                        }
                                    });
                                });
                        }
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

                                    // Filter files to include only those starting with "entity_" json files
                                    let mut entity_files: Vec<String> = files
                                        .into_iter()
                                        .filter(|file| {
                                            file.starts_with("entity_") && file.ends_with(".json")
                                        })
                                        .collect();

                                    entity_files.sort_by_key(|file| {
                                        FileManagement::extract_id_from_file(file)
                                    });

                                    for file in entity_files {
                                        if let Some(entity_id) =
                                            FileManagement::extract_id_from_file(&file)
                                        {
                                            // Check if it's already in the ecs
                                            if !self.ecs.entity_exists_by_id(entity_id) {
                                                // Create the entity with the given ID if it doesn't exist
                                                // self.ecs.create_entity_by_id(entity_id);
                                                // Load the entity with the given ID from json file
                                                if let Err(err) = self.load_entity(entity_id) {
                                                    self.print_to_terminal(&err);
                                                }
                                            }

                                            let is_highlighted = self.highlighted_entity == Some(entity_id);

                                            let button =
                                                egui::Button::new(format!("Entity {}", entity_id))
                                                    .fill(if is_highlighted {
                                                        egui::Color32::from_rgb(200, 200, 255)
                                                    } else {
                                                        egui::Color32::from_rgb(240, 240, 240)
                                                    });

                                            let button_response = ui.add(button);

                                            // Handle right-click on button
                                            button_response.context_menu(|ui| {
                                                if ui.button("Delete").clicked() {
                                                    self.delete_entity_by_file(&file);
                                                    ui.close_menu();
                                                }
                                            });

                                            if button_response.clicked() {
                                                self.highlighted_entity = Some(entity_id);
                                                self.print_to_terminal(&format!(
                                                    "Clicked on entity ID: {}",
                                                    entity_id
                                                ));
                                            }
                                        }
                                    }

                                    // self.print_sorted_entity_ids_to_terminal();
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

                        if let Some(script_id) = self.highlighted_script {
                            ui.label(format!("Script ID: {}", script_id));

                            // Text editor for the script content
                            egui::ScrollArea::vertical()
                                .auto_shrink([false; 2])
                                .show(ui, |ui| {
                                    ui.text_edit_multiline(&mut self.current_script_content);


                                // Save and reload buttons
                                ui.horizontal(|ui| {
                                    // Save content to file
                                    if ui.button("Save").clicked() {
                                        let script_file_path = format!("{}/scripts/script_{}.lua", self.project_path, script_id);
                                        match FileManagement::save_to_file(&self.current_script_content, &script_file_path) {
                                            Ok(_) => {
                                                self.print_to_terminal(&format!("Script {} saved successfully.", script_id));
                                            }
                                            Err(err) => {
                                                self.print_to_terminal(&format!(
                                                    "Failed to save script {}: {}",
                                                    script_id, err
                                                ));
                                            }
                                        }
                                    }

                                    // Reload content from file
                                    if ui.button("Reload").clicked() {
                                        let script_file_path = format!("{}/scripts/script_{}.lua", self.project_path, script_id);
                                        match FileManagement::load_file_content(&script_file_path) {
                                            Ok(content) => {
                                                self.current_script_content = content;
                                                self.print_to_terminal(&format!(
                                                    "Reloaded script {} successfully.",
                                                    script_id
                                                ));
                                            }
                                            Err(err) => {
                                                self.print_to_terminal(&format!(
                                                    "Failed to reload script {}: {}",
                                                    script_id, err
                                                ));
                                            }
                                        }
                                    }
                                });
                            });

                        } else {
                            ui.label("Inspect and modify the attributes of the script");
                        }

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

                                    // Filter files to include only those starting with "script_" lua files
                                    let mut script_files: Vec<String> = files
                                        .into_iter()
                                        .filter(|file| {
                                            file.starts_with("script_") && file.ends_with(".lua")
                                        })
                                        .collect();

                                    // Sort files by ID
                                    script_files.sort_by_key(|file| {
                                        FileManagement::extract_id_from_file(file)
                                    });

                                    for file in script_files {
                                        if let Some(script_id) = FileManagement::extract_id_from_file(&file)
                                        {

                                            if script_id >= self.next_script_id {
                                                self.next_script_id = script_id + 1;
                                            }

                                            let is_highlighted = self.highlighted_script == Some(script_id);

                                            let button =
                                                egui::Button::new(format!("Script {}", script_id))
                                                    .fill(if is_highlighted {
                                                        egui::Color32::from_rgb(200, 200, 255)
                                                    } else {
                                                        egui::Color32::from_rgb(240, 240, 240)
                                                    });

                                            let button_response = ui.add(button);

                                            // Handle right-click on button
                                            button_response.context_menu(|ui| {
                                                if ui.button("Delete").clicked() {
                                                    self.delete_script_by_file(&file);
                                                    ui.close_menu();
                                                }
                                            });

                                            if button_response.clicked() {
                                                self.highlighted_script = Some(script_id);

                                                // Load script content from file
                                                let script_file_path = format!("{}/scripts/script_{}.lua", self.project_path, script_id);
                                                match FileManagement::load_file_content(&script_file_path) {
                                                    Ok(content) => {
                                                        self.current_script_content = content;
                                                    }
                                                    Err(err) => {
                                                        self.print_to_terminal(&format!(
                                                            "Failed to load script content for ID {}: {}",
                                                            script_id, err
                                                        ));
                                                    }
                                                }

                                                self.print_to_terminal(&format!(
                                                    "Clicked on script ID: {}",
                                                    script_id
                                                ));

                                            }
                                        }
                                    }
                                });
                        }
                    });
            });

        // Central panel
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Scene");

            // Total width of buttons (hardcoded)
            let total_buttons_width = 167.0;

            // Calculate the padding for centering buttons
            let panel_width = ui.available_width();
            let padding = (panel_width - total_buttons_width) / 2.0;

            ui.horizontal(|ui| {
                ui.add_space(padding.max(0.0));

                // Play button
                if ui.button("▶ Play").clicked() {
                    self.print_to_terminal("Play button clicked.");
                }

                // Pause button
                if ui.button("⏸ Pause").clicked() {
                    self.print_to_terminal("Pause button clicked.");
                }

                // Step button
                if ui.button("⏭ Step").clicked() {
                    self.print_to_terminal("Step button clicked.");
                }
            });

            ui.add_space(20.0);
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

    /// Add a new entity to ecs and create the corresponding json file
    pub fn add_entity(&mut self) {

        let entity = self.ecs.create_entity();
        let serialized_entity = entity.to_json();

        // File path for the entity json file
        let entity_file_path = format!("{}/entities/entity_{}.json", self.project_path, entity.id);

        // Save the json to file
        if let Err(err) = FileManagement::save_to_file(&serialized_entity, &entity_file_path) {
            self.print_to_terminal(&format!("Failed to save entity {}: {}", entity.id, err));
        } else {
            self.print_to_terminal(&format!(
                "Entity {} saved successfully at {}",
                entity.id, entity_file_path
            ));
        }
    }

    /// Delete an entity by removing it from ecs and deleting the corresponding json file
    pub fn delete_entity_by_file(&mut self, file_name: &str) {
        if let Some(entity_id) = FileManagement::extract_id_from_file(file_name) {
            if let Some(entity) = self.ecs.entities.get(&entity_id).cloned() {
                // Remove the entity from ecs
                self.ecs.delete_entity(entity);

                // File path of the entity
                let file_path = format!("{}/entities/{}", self.project_path, file_name);

                // Delete the file
                match FileManagement::delete_file(&file_path) {
                    Ok(_) => {
                        self.print_to_terminal(&format!("Deleted entity and file: {}", file_name));
                    }
                    Err(e) => {
                        self.print_to_terminal(&format!("Failed to delete entity file: {}", e));
                    }
                }
            } else {
                self.print_to_terminal(&format!(
                    "Entity with ID {} does not exist in ECS",
                    entity_id
                ));
            }
        } else {
            self.print_to_terminal(&format!("Invalid entity file name: {}", file_name));
        }
    }

    /// Print the list of sorted entity IDs to the terminal
    pub fn print_sorted_entity_ids_to_terminal(&mut self) {
        let mut entity_ids: Vec<usize> = self.ecs.entities.keys().cloned().collect();
        entity_ids.sort(); // Sort the IDs as numbers
        let message = format!(
            "Entity IDs: [{}]",
            entity_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        self.print_to_terminal(&message);
    }

        /// Save the entity to its corresponding json file
    pub fn update_entity(&mut self, entity_id: usize) {
        if let Some(entity) = self.ecs.get_entity_by_id(entity_id) {

            let serialized_entity = entity.to_json();

            // File path for the entity json file
            let entity_file_path =
                format!("{}/entities/entity_{}.json", self.project_path, entity.id);

            // Save the updated json to file
            match FileManagement::save_to_file(&serialized_entity, &entity_file_path) {
                Ok(_) => {
                    self.print_to_terminal(&format!(
                        "Entity {} updated successfully at {}",
                        entity.id, entity_file_path
                    ));
                }
                Err(err) => {
                    self.print_to_terminal(&format!(
                        "Failed to update entity {}: {}",
                        entity.id, err
                    ));
                }
            }
        } else {
            self.print_to_terminal(&format!(
                "Failed to update: Entity with ID {} not found.",
                entity_id
            ));
        }
    }

    /// Load an entity by its ID from the corresponding json file
    pub fn load_entity(&mut self, entity_id: usize) -> Result<(), String> {

        let entity_file_path = format!("{}/entities/entity_{}.json", self.project_path, entity_id);

        let file_content = std::fs::read_to_string(&entity_file_path)
            .map_err(|err| format!("Failed to read file '{}': {}", entity_file_path, err))?;

        // Deserialize the json into an Entity object
        let entity: Entity = serde_json::from_str(&file_content).map_err(|err| {
            format!(
                "Failed to parse JSON from file '{}': {}",
                entity_file_path, err
            )
        })?;

        // Check if the entity already exists in ecs
        if self.ecs.entity_exists_by_id(entity_id) {
            return Err(format!(
                "Entity with ID {} already exists in ECS.",
                entity_id
            ));
        }

        // Add the entity to ecs by id
        self.ecs.insert_entity_by_id(entity_id, entity);

        self.print_to_terminal(&format!("Entity {} loaded successfully.", entity_id));
        Ok(())
    }

    /// Add a new script json file
    pub fn add_script(&mut self) {

        let script_id = self.next_script_id;

        // File path for the script json file
        let script_file_path = format!("{}/scripts/script_{}.lua", self.project_path, script_id);

        // Save the json to file
        if let Err(err) = FileManagement::save_to_file("", &script_file_path) {
            self.print_to_terminal(&format!("Failed to save script {}: {}", script_id, err));
        } else {
            self.print_to_terminal(&format!(
                "Script {} saved successfully at {}",
                script_id, script_file_path
            ));

            self.next_script_id += 1;
        }
    }

    /// Delete a script file
    pub fn delete_script_by_file(&mut self, file_name: &str) {
        let file_path = format!("{}/scripts/{}", self.project_path, file_name);

        match FileManagement::delete_file(&file_path) {
            Ok(_) => {
                self.print_to_terminal(&format!("Deleted script file: {}", file_name));
                self.highlighted_script = None;
            }
            Err(e) => {
                self.print_to_terminal(&format!("Failed to delete script file: {}", e));
            }
        }
    }

    /// Add the import menu in UI
    fn show_import_menu(&mut self, ui: &mut egui::Ui) {
        let asset_types = [
            ("Font", &["ttf", "otf"][..], "assets/fonts"),
            ("Image", &["png", "jpg", "jpeg", "bmp"][..], "assets/images"),
            ("Sound", &["mp3", "wav", "ogg"][..], "assets/sounds"),
            ("Video", &["mp4", "avi", "mkv", "webm"][..], "assets/videos"),
        ];

        ui.menu_button("Import...", |ui| {

            ui.add_enabled_ui(self.load_project, |ui| {
                for (name, extensions, folder) in &asset_types {
                    if ui.button(*name).clicked() {
                        if let Some(file_path) = FileDialog::new().add_filter((*name).to_string(), *extensions).pick_file() {
                            match FileManagement::import_asset(
                                file_path.to_str().unwrap_or(""),
                                &format!("{}/{}", self.project_path, folder),
                            ) {
                                Ok(msg) => self.print_to_terminal(&msg),
                                Err(err) => self.print_to_terminal(&err),
                            }
                        }
                        ui.close_menu();
                    }
                }
            });
        });
    }
}
