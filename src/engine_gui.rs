use crate::ecs::{Attribute, AttributeValueType, Entity, EntityManager};
use crate::project_manager::FileManagement;
use crate::render_engine::{RenderEngine, Animation, RenderObject, Scene, RenderLayer, Transform};
use crate::input_handler::{InputHandler, InputContext};
use eframe::egui;
use rfd::FileDialog;

pub struct EngineGui {
    ecs: EntityManager,
    render_engine: RenderEngine,
    show_new_project_popup: bool,  // Track if the pop-up should be shown
    show_open_project_popup: bool, // Track if the pop-up should be shown
    load_project: bool,            // Track if the project should be loaded
    project_name: String,          // Store the project name input
    project_path: String,          // Store the project path input
    terminal_output: String,       // Store the terminal output
    // entities panel
    highlighted_entity: Option<usize>,
    highlighted_attribute: Option<(usize, String)>,
    editing_attribute: Option<String>,
    new_entity_name: String,
    show_entity_rename_popup: bool,
    // entity inspector panel
    show_add_attribute_popup: bool,
    show_reorder_attribute_popup: bool,
    add_attribute_popup_error_msg: String,
    selected_attribute_type: AttributeValueType,
    new_attribute_name: String,
    new_attribute_value: String,
    // scripts panel
    highlighted_script: Option<usize>,
    next_script_id: usize,
    current_script_content: String,
    // scene panel
    running: bool,
    paused: bool,
    delta_time: f32,
    scene: Option<Scene>,
    input_handler: InputHandler,
}

impl Default for EngineGui {
    fn default() -> Self {
        Self {
            ecs: EntityManager::new(),
            render_engine: RenderEngine::new(),
            show_new_project_popup: false,
            show_open_project_popup: false,
            load_project: false,
            project_name: String::new(),
            project_path: String::new(),
            terminal_output: String::new(),
            // entities panel
            highlighted_entity: None,
            highlighted_attribute: None,
            editing_attribute: None,
            new_entity_name: String::new(),
            show_entity_rename_popup: false,
            // entity inspector panel
            show_add_attribute_popup: false,
            show_reorder_attribute_popup: false,
            add_attribute_popup_error_msg: String::new(),
            selected_attribute_type: AttributeValueType::String(String::new()),
            new_attribute_name: String::new(),
            new_attribute_value: String::new(),
            // scripts panel
            highlighted_script: None,
            next_script_id: 0,
            current_script_content: String::new(),
            // scene panel
            running: false,
            paused: false,
            delta_time: 0.0,
            scene: None,
            input_handler: InputHandler::new(),
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

        if self.load_project && self.running {
            self.run_game(ctx);
        }

        // Update the central panel to use egui-wgpu rendering
        egui::CentralPanel::default().show(ctx, |ui| {
            let viewport_rect = ui.available_rect_before_wrap();

            // Play/Pause/Reset buttons
            let button_width = 60.0;
            let button_height = 20.0;
            let button_group_width = (button_width * 3.0) + 10.0;
            let center_x = viewport_rect.center().x - (button_group_width / 2.0);

            egui::Area::new("floating_buttons".into())
                .fixed_pos(egui::pos2(center_x, viewport_rect.min.y + 10.0))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.set_width(button_group_width);

                        if ui.add_sized([button_width, button_height], egui::Button::new("▶ Play")).clicked() {
                            self.running = true;
                        }

                        if ui.add_sized([button_width, button_height], egui::Button::new("⏸ Pause")).clicked() {
                            self.running = false;
                        }

                        if ui.add_sized([button_width, button_height], egui::Button::new("⏹ Reset")).clicked() {
                            self.running = false;
                        }
                    });
                });

            // Run the game if we're in running state
            if self.running {
                self.run_game(ctx);
            }
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
                    .max_height(ui.available_height() - ui.spacing().item_spacing.y)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.label(&self.terminal_output);
                    });
            });
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


                        ui.add_enabled(self.load_project, egui::Button::new("Build And Run"))
                            .clicked()
                            .then(|| {
                                let project_path = self.project_path.clone();
                                match FileManagement::build_and_run_project(&project_path, self) {
                                    Ok(_) => self.print_to_terminal("Build and run succeeded!"),
                                    Err(e) => self.print_to_terminal(&format!("Error: {}", e)),
                                }
                            });

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
                self.show_entity_inspector_panel(ctx, ui, secondary_panel_height);

                // Bottom section
                self.show_entity_panel(ctx, ui, secondary_panel_height);

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
                    .frame(egui::Frame::none().inner_margin(egui::Margin::same(5.0)))
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
                    .frame(egui::Frame::none().inner_margin(egui::Margin::same(5.0)))
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

    pub fn register_texture_with_egui(&mut self, ctx: &egui::Context) -> Option<egui::TextureId> {
        // Create a test image - solid red 100x100 pixels
        let width = 100;
        let height = 100;
        let mut pixels = vec![0u8; width * height * 4];

        // Fill with red
        for pixel in pixels.chunks_mut(4) {
            pixel[0] = 255; // R
            pixel[1] = 0;   // G
            pixel[2] = 0;   // B
            pixel[3] = 255; // A
        }

        // Create the color image
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [width, height],
            &pixels
        );

        // Load the texture
        let texture = ctx.load_texture(
            "test_texture",
            color_image,
            egui::TextureOptions::NEAREST
        );

        self.print_to_terminal("Created new egui texture");
        Some(texture.id())
    }

    // Main game loop
    pub fn run_game(&mut self, ctx: &egui::Context) {
        if self.running {
            self.input_handler.set_context(InputContext::Game);

            // Calculate delta time
            let now = std::time::Instant::now();
            self.delta_time = now.duration_since(self.render_engine.last_frame_time).as_secs_f32();
            self.render_engine.last_frame_time = now;

            // Initialize game if needed
            if self.scene.is_none() {
                self.initialize_game(ctx);
            }

            // Update input state
            ctx.input(|input| {
                self.input_handler.handle_input(input);
            });

            // Handle input - This is where our camera controls happen
            self.handle_input();

            // Update game state
            if let Some(scene) = &mut self.scene {
                scene.update(self.delta_time);
            }

            // Render
            self.render(ctx);

            ctx.request_repaint();
        } else {
            self.input_handler.set_context(InputContext::Editor);
        }
    }

    // Initialize game resources
    fn initialize_game(&mut self, ctx: &egui::Context) {
        let mut scene = Scene::new();

        // Add test animation
        if let Some(animation) = self.create_test_animation(ctx) {
            scene.add_object(
                RenderObject::Animated {
                    animation,
                    transform: Transform::new((300.0, 300.0)),
                },
                RenderLayer::Game  // Specify the layer
            );
            self.print_to_terminal("Added rotating rectangle to scene");
        }

        // Add test static object
        if let Some(texture) = self.create_test_object(ctx) {
            scene.add_object(
                RenderObject::Static {
                    texture,
                    transform: Transform::new((300.0, 100.0)),
                },
                RenderLayer::Game  // Specify the layer
            );
            self.print_to_terminal("Added static checker pattern to scene");
        }

        self.scene = Some(scene);
    }

    // Handle user input
    fn handle_input(&mut self) {
        if let Some(scene) = &mut self.scene {
            // Increase camera movement speed significantly
            let camera_speed = 200.0 * self.delta_time;  // Increased from 5.0 to 200.0

            // Only handle game input when running
            if self.running {
                if self.input_handler.is_key_pressed(egui::Key::W) {
                    scene.move_camera((0.0, -camera_speed));
                }
                if self.input_handler.is_key_pressed(egui::Key::S) {
                    scene.move_camera((0.0, camera_speed));
                }
                if self.input_handler.is_key_pressed(egui::Key::A) {
                    scene.move_camera((-camera_speed, 0.0));
                }
                if self.input_handler.is_key_pressed(egui::Key::D) {
                    scene.move_camera((camera_speed, 0.0));
                }

                // Adjust zoom speed and invert the zoom direction
                let zoom_speed = 2.0 * self.delta_time;  // Increased from 1.0 to 2.0
                if self.input_handler.is_key_pressed(egui::Key::Q) {
                    scene.zoom_camera(zoom_speed);  // Zoom out makes things smaller (positive scale)
                }
                if self.input_handler.is_key_pressed(egui::Key::E) {
                    scene.zoom_camera(-zoom_speed);  // Zoom in makes things bigger (negative scale)
                }

                let rotation_speed = 2.0 * self.delta_time;  // Increased rotation speed too
                if self.input_handler.is_key_pressed(egui::Key::R) {
                    scene.rotate_camera(-rotation_speed);
                }
                if self.input_handler.is_key_pressed(egui::Key::F) {
                    scene.rotate_camera(rotation_speed);
                }
            }
        }
    }

    // Update physics
    fn update_physics(&mut self) {
        // TODO: Collision detection, movement, etc.
    }

    // Update game state
    fn update_game_state(&mut self) {
        if let Some(scene) = &mut self.scene {
            scene.update(self.delta_time);
        }
    }

    // Render everything
    fn render(&self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(scene) = &self.scene {
                // Get batches sorted by layer
                let mut batches = scene.prepare_batches();
                batches.sort_by_key(|batch| batch.layer);

                // Render each batch
                for batch in batches {
                    for instance in &batch.instances {
                        let size = egui::vec2(
                            batch.texture.size()[0] as f32 * instance.transform.scale.0 * scene.camera.zoom,
                            batch.texture.size()[1] as f32 * instance.transform.scale.1 * scene.camera.zoom
                        );

                        let (x, y) = scene.camera.transform_point(instance.transform.position);
                        let pos = egui::pos2(x, y);

                        ui.put(
                            egui::Rect::from_min_size(pos, size),
                            egui::Image::new((batch.texture.id(), size))
                                .uv(instance.uv_rect)
                                .rotate(instance.transform.rotation, egui::vec2(0.5, 0.5))
                                .tint(egui::Color32::from_rgba_unmultiplied(
                                    (instance.color[0] * 255.0) as u8,
                                    (instance.color[1] * 255.0) as u8,
                                    (instance.color[2] * 255.0) as u8,
                                    (instance.color[3] * 255.0) as u8,
                                ))
                        );
                    }
                }
            }
        });
    }

    pub fn create_test_object(&mut self, ctx: &egui::Context) -> Option<egui::TextureHandle> {
        let square_size = 10;  // Size of each checker square in pixels
        let squares = 5;       // Number of squares in each row/column
        let width = square_size * squares;   // Total width in pixels
        let height = square_size * squares;  // Total height in pixels

        let mut pixels = vec![0u8; width * height * 4];

        // Fill the pixels first
        for y in 0..height {
            for x in 0..width {
                let i = (y * width + x) * 4;
                // Determine which square this pixel belongs to
                let square_x = x / square_size;
                let square_y = y / square_size;
                let is_checker = (square_x + square_y) % 2 == 0;

                if is_checker {
                    pixels[i] = 255;     // R
                    pixels[i + 1] = 0;   // G
                    pixels[i + 2] = 0;   // B
                    pixels[i + 3] = 255; // A

                    // Debug print for first square
                    if square_x == 0 && square_y == 0 && x < 2 && y < 2 {
                        self.print_to_terminal(&format!(
                            "Red Square (0,0) Pixel ({}, {}): rgba=[{},{},{},{}]",
                            x, y,
                            pixels[i], pixels[i+1], pixels[i+2], pixels[i+3]
                        ));
                    }
                } else {
                    pixels[i] = 0;       // R
                    pixels[i + 1] = 0;   // G
                    pixels[i + 2] = 255; // B
                    pixels[i + 3] = 255; // A

                    // Debug print for second square
                    if square_x == 1 && square_y == 0 && x >= square_size && x < square_size + 2 {
                        self.print_to_terminal(&format!(
                            "Blue Square (1,0) Pixel ({}, {}): rgba=[{},{},{},{}]",
                            x, y,
                            pixels[i], pixels[i+1], pixels[i+2], pixels[i+3]
                        ));
                    }
                }
            }
        }

        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [width, height],
            &pixels
        );

        Some(ctx.load_texture(
            "test_pattern",
            color_image,
            egui::TextureOptions::NEAREST
        ))
    }

    pub fn create_test_animation(&mut self, ctx: &egui::Context) -> Option<Animation> {
        let frames_count = 120;  // 120 frames for very smooth rotation (3° per frame)
        let mut frames = Vec::new();

        // Rectangle dimensions
        let width = 100;
        let height = 50;
        let canvas_size = 120;  // Square canvas to allow rotation

        for i in 0..frames_count {
            let mut pixels = vec![0u8; canvas_size * canvas_size * 4];

            // Calculate rotation angle for this frame (360° / 60 frames = 6° per frame)
            let angle = (i as f32 * (360.0/frames_count as f32)).to_radians();
            let cos_a = angle.cos();
            let sin_a = angle.sin();

            // Draw rotated rectangle
            for y in 0..canvas_size {
                for x in 0..canvas_size {
                    // Center coordinates
                    let cx = x as f32 - canvas_size as f32 / 2.0;
                    let cy = y as f32 - canvas_size as f32 / 2.0;

                    // Rotate point
                    let rx = cx * cos_a - cy * sin_a;
                    let ry = cx * sin_a + cy * cos_a;

                    // Check if point is inside rectangle
                    let is_inside = rx.abs() < width as f32 / 2.0 &&
                                  ry.abs() < height as f32 / 2.0;

                    let pixel_idx = (y * canvas_size + x) * 4;
                    if is_inside {
                        pixels[pixel_idx] = 255;     // R
                        pixels[pixel_idx + 1] = 0;   // G
                        pixels[pixel_idx + 2] = 0;   // B
                        pixels[pixel_idx + 3] = 255; // A
                    }
                }
            }

            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [canvas_size, canvas_size],
                &pixels
            );

            frames.push(ctx.load_texture(
                &format!("rotation_frame_{}", i),
                color_image,
                egui::TextureOptions::NEAREST
            ));
        }

        // Set frame duration for 60 FPS (1/60 ≈ 0.0167 seconds per frame)
        Some(Animation::new(frames, 1.0/60.0))  // One complete rotation per second
    }

    /// Entity Inspector Panel
    pub fn show_entity_inspector_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, secondary_panel_height: f32) {
        egui::TopBottomPanel::top("entity_inspector")
            .resizable(false)
            .exact_height(secondary_panel_height)
            .frame(egui::Frame::none().inner_margin(egui::Margin::same(5.0)))
            .show_inside(ui, |ui| {
                ui.heading("Entity Inspector");

                if let Some(selected_id) = self.highlighted_entity {
                    self.show_entity_subheading(ui, selected_id);

                    ui.separator();

                    if let Some((attributes, attribute_order)) = self.ecs.get_attributes_by_entity_id(selected_id) {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false; 2])
                            .max_height(secondary_panel_height -  - ui.spacing().item_spacing.y)
                            .show(ui, |ui| {
                                self.show_entity_attributes(ui, selected_id, &attributes, &attribute_order);
                            });
                    } else {
                        ui.label("Selected entity not found.");
                    }
                } else {
                    ui.label("Inspect and modify the attributes of the entity");
                }

                if self.show_add_attribute_popup {
                    self.show_add_attribute_popup(ctx);
                }

                if self.show_reorder_attribute_popup {
                    self.show_reorder_attribute_popup(ctx);
                }


            });
    }

    /// Show the id and add attribute button as subheading
    fn show_entity_subheading(&mut self, ui: &mut egui::Ui, selected_id: usize) {
        egui::Grid::new("entity_inspector_subheading_grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(false)
            .show(ui, |ui| {
                ui.label(format!("Entity ID: {}", selected_id));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {

                    if ui.button("Add").clicked() {
                        self.show_add_attribute_popup = true;
                    }

                    if ui.button("Reorder").clicked() {
                        self.show_reorder_attribute_popup = true;
                    }

                });
                ui.end_row();
            });
    }

    /// Show entity attributes in two columns
    fn show_entity_attributes(
        &mut self,
        ui: &mut egui::Ui,
        selected_id: usize,
        attributes: &std::collections::HashMap<String, Attribute>,
        attribute_order: &[String],
    ) {

        egui::Grid::new("entity_inspector_attributes_grid")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                for key in attribute_order {
                    if let Some(attribute) = attributes.get(key) {
                        self.show_attribute_key(ui, key, selected_id, 10); // Max 10 characters for truncation

                        // Show attribute value with editable field
                        if let Some((editing_entity, editing_key)) = &self.highlighted_attribute {
                            if *editing_entity == selected_id && *editing_key == *key {
                                // Display text input for editing
                                let mut new_value = if let Some(editing_value) = &self.editing_attribute {
                                    if editing_value.is_empty() {
                                        format!("{}", attribute.value_type)
                                    } else {
                                        editing_value.clone()
                                    }
                                } else {
                                    format!("{}", attribute.value_type)
                                };

                                let field_response = ui.text_edit_singleline(&mut new_value);

                                if field_response.changed() {
                                    self.editing_attribute = Some(new_value.clone());
                                }
                                if field_response.lost_focus() {
                                    if ui.ctx().input(|i| i.key_pressed(egui::Key::Enter)) {
                                        // Modify attribute
                                        match self.ecs.modify_attribute_by_entity_id(
                                            selected_id,
                                            key.clone(),
                                            attribute.value_type.clone(),
                                            new_value.clone(),
                                        ) {
                                            Ok(_) => {
                                                self.update_entity(selected_id);
                                                self.print_to_terminal(&format!(
                                                    "Updated attribute '{}' for entity {}.",
                                                    key, selected_id
                                                ));
                                                self.highlighted_attribute = None;
                                                self.editing_attribute = None;
                                            }
                                            Err(err_msg) => {
                                                self.print_to_terminal(&format!(
                                                    "Error updating attribute '{}': {}",
                                                    key, err_msg
                                                ));
                                                self.highlighted_attribute = None;
                                                self.editing_attribute = None;
                                            }
                                        }
                                    } else if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
                                        self.highlighted_attribute = None;
                                        self.editing_attribute = None;
                                    } else {
                                        self.highlighted_attribute = None;
                                        self.editing_attribute = None;
                                    }
                                }
                            } else {
                                self.show_editable_value(ui, key, attribute, selected_id);
                            }
                        } else {
                            self.show_editable_value(ui, key, attribute, selected_id);
                        }
                        ui.end_row();
                    }
            }
        });
    }

    /// Show attribute key with truncation and tooltip if the key is too long.
    /// Can't use ui.add_sized function, which forces the text center. We want it aligns right.
    fn show_attribute_key(&mut self, ui: &mut egui::Ui, key: &str, selected_id: usize, max_chars: usize) {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            let full_text = format!("{}:", key);
            let display_text = if full_text.len() > max_chars {
                format!("{}...:", &full_text[..max_chars])
            } else {
                full_text.clone()
            };

            // Make the label has hyperlink style
            let hyperlink_style = egui::RichText::new(&display_text)
                .underline()
                .color(egui::Color32::from_rgb(0, 0, 255));

            let label = ui.add(egui::Label::new(hyperlink_style).sense(egui::Sense::click()));

            if label.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            if label.hovered() && full_text.len() > max_chars {
                label.show_tooltip_ui(|ui| {
                    ui.label(key);
                });
            }

            // Handle right-click on button
            label.context_menu(|ui| {
                if ui.button("Delete").clicked() {
                    self.ecs
                        .delete_attribute_by_entity_id(selected_id, &key.to_string());
                    self.update_entity(selected_id);
                    self.print_to_terminal(&format!(
                        "Deleted attribute '{}' for entity {}.",
                        key, selected_id
                    ));
                    ui.close_menu();
                }
            });

            if label.clicked() {
                // Open popup for editing the attribute
                // self.show_edit_attribute_popup = true;
            }
        });
    }

    /// Show attribute key with truncation and tooltip if the key is too long.
    /// Using ui.add_sized function, which forces the text center.
    fn show_attribute_value(&self, ui: &mut egui::Ui, attribute: &Attribute) {
        ui.add_sized(
            [ui.available_width(), ui.available_height()],
            egui::Label::new(format!("{}", attribute.value_type)).truncate(),
        );
    }

    /// Show attribute key with truncation and tooltip if the key is too long.
    /// Using ui.add_sized function, which forces the text center.
    /// The field is editable when double-clicked.
    fn show_editable_value(
        &mut self,
        ui: &mut egui::Ui,
        key: &str,
        attribute: &Attribute,
        selected_id: usize,
    ) {
        let label = ui.add_sized(
            [ui.available_width(), ui.available_height()],
            egui::Label::new(format!("{}", attribute.value_type)).truncate()
                .sense(egui::Sense::click()),
        );

        if label.double_clicked() {
            self.highlighted_attribute = Some((selected_id, key.to_string()));
            self.editing_attribute = Some(format!("{}", attribute.value_type));
        }
    }

    /// Displays a popup to add a new attribute to the selected entity.
    /// The attribute type is validated before being added.
    fn show_add_attribute_popup(&mut self, ctx: &egui::Context) {

        egui::Window::new("Add Attribute")
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label("Attribute Name:");
                ui.text_edit_singleline(&mut self.new_attribute_name);

                ui.label("Attribute Type:");
                egui::ComboBox::from_label("Type")
                    .selected_text(format!("{:?}", self.selected_attribute_type))
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
                            let attribute_name = self.new_attribute_name.trim().to_string();
                            let attribute_value = self.new_attribute_value.trim().to_string();

                            // check if the attribute name is empty
                            if attribute_name.is_empty() {
                                // self.print_to_terminal("Attribute name cannot be empty.");
                                self.add_attribute_popup_error_msg = "Attribute name cannot be empty.".to_string();
                                return;
                            }

                            // check if the attribute name exists
                            if let Some(entity) = self.ecs.get_entity_by_id(selected_id) {
                                if self.ecs.attribute_exists(entity, &attribute_name) {
                                    // self.print_to_terminal("Attribute name already exists.");
                                    self.add_attribute_popup_error_msg = "Attribute name already exists.".to_string();
                                    return;
                                }
                            } else {
                                // self.print_to_terminal("Selected entity not found.");
                                self.add_attribute_popup_error_msg = "Selected entity not found.".to_string();
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
                                    self.print_to_terminal("Attribute added successfully.");
                                    self.new_attribute_name.clear();
                                    self.new_attribute_value.clear();
                                    self.show_add_attribute_popup = false;
                                    self.add_attribute_popup_error_msg.clear();
                                }
                                Err(err) => {
                                    // self.print_to_terminal(&format!("Error adding attribute: {}", err));
                                    self.add_attribute_popup_error_msg = format!("Error adding attribute: {}", err);
                                }
                            }
                        } else {
                            // self.print_to_terminal("No entity selected.");
                            self.add_attribute_popup_error_msg = "No entity selected.".to_string();
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        self.show_add_attribute_popup = false;
                        self.add_attribute_popup_error_msg.clear();
                    }
                });

                // Display error message at the bottom of the popup
                if !self.add_attribute_popup_error_msg.is_empty() {
                    ui.add_space(10.0);
                    ui.colored_label(egui::Color32::RED, &self.add_attribute_popup_error_msg);
                }

            });
    }

    /// Popup window dor drag and drop reorder for attributes
    fn show_reorder_attribute_popup(&mut self, ctx: &egui::Context) {
        if let Some(selected_id) = self.highlighted_entity {
            if let Some((_, attribute_order)) = self.ecs.get_attributes_by_entity_id(selected_id) {
                let mut rows = attribute_order.clone();

                egui::Window::new("Reorder Attributes")
                    .resizable(false)
                    .collapsible(false)
                    .show(ctx, |ui| {
                        let mut from = None;
                        let mut to = None;

                        let frame = egui::Frame::default().inner_margin(4.0);

                        ui.dnd_drop_zone::<usize, ()>(frame, |ui| {
                            ui.set_min_size(egui::vec2(64.0, 100.0));
                            for (row_idx, item) in rows.iter().enumerate() {
                                let item_id = egui::Id::new(("entity_attribute", row_idx));

                                let response = ui
                                    .dnd_drag_source(item_id, row_idx, |ui| {
                                        ui.label(item);
                                    })
                                    .response;

                                // Detect drops onto this item:
                                if let (Some(pointer), Some(hovered_payload)) = (
                                    ui.input(|i| i.pointer.interact_pos()),
                                    response.dnd_hover_payload::<usize>(),
                                ) {
                                    let rect = response.rect;

                                    // Preview insertion:
                                    let stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                                    let insert_row_idx = if *hovered_payload == row_idx {
                                        // We are dragged onto ourselves
                                        ui.painter().hline(rect.x_range(), rect.center().y, stroke);
                                        row_idx
                                    } else if pointer.y < rect.center().y {
                                        // Above us
                                        ui.painter().hline(rect.x_range(), rect.top(), stroke);
                                        row_idx
                                    } else {
                                        // Below us
                                        ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                                        row_idx + 1
                                    };

                                    if let Some(dragged_payload) = response.dnd_release_payload::<usize>() {
                                        // The user dropped onto this item.
                                        from = Some(*dragged_payload);
                                        to = Some(insert_row_idx);
                                    }
                                }
                            }
                        });

                        if let (Some(from), Some(mut to)) = (from, to) {
                            if from == to {
                                // Dragging within the same column.
                                // Adjust row index if we are re-ordering:
                                to -= (from < to) as usize;
                            }

                            let item = rows.remove(from);
                            to = to.min(rows.len());
                            rows.insert(to, item);

                        }

                        // update entity
                        self.ecs
                            .update_entity_attribute_order(selected_id, rows.clone());
                        self.update_entity(selected_id);

                        if ui.button("Done").clicked() {
                            self.show_reorder_attribute_popup = false;
                        }

                    });
            } else {
                self.print_to_terminal(&format!(
                    "Failed to retrieve attributes for entity {}.",
                    selected_id
                ));
            }
        } else {
            self.print_to_terminal("No entity selected for reordering.");
        }
    }



    /// Entity panel
    fn show_entity_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, secondary_panel_height: f32) {
        let entity_folder_path = format!("{}/entities", self.project_path);
        let item_spacing = ctx.style().spacing.item_spacing;
        egui::TopBottomPanel::bottom("entity")
            .resizable(false)
            .exact_height(secondary_panel_height)
            .show_separator_line(false)
            .frame(egui::Frame::none().inner_margin(egui::Margin::same(5.0)))
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

                                    let mut display_name:String = self.ecs.get_entity_by_id(entity_id).unwrap().name.clone();

                                    if display_name.is_empty() {
                                        display_name = format!("Entity {}", entity_id);
                                    }

                                    let button =
                                        egui::Button::new(display_name)
                                            .min_size(egui::vec2(ui.available_width(), ui.style().spacing.item_spacing.y))
                                            .fill(if is_highlighted {
                                                egui::Color32::from_rgb(200, 200, 255)
                                            } else {
                                                egui::Color32::from_rgb(240, 240, 240)
                                            });

                                    let button_response = ui.add(button);

                                    // Handle right-click on button
                                    button_response.context_menu(|ui| {

                                        self.highlighted_entity = Some(entity_id);

                                        if ui.button("Rename").clicked() {

                                            self.show_entity_rename_popup = true;
                                            ui.close_menu();
                                        }
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

                if self.show_entity_rename_popup {
                    self.show_entity_rename_popup(ctx);
                }

            });
    }

    /// Entity rename popup
    fn show_entity_rename_popup(&mut self, ctx: &egui::Context) {

        egui::Window::new("Rename Entity")
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label("Entity Name:");
                ui.text_edit_singleline(&mut self.new_entity_name);

                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        if let Some(selected_id) = self.highlighted_entity {
                            let entity_name = self.new_entity_name.trim().to_string();

                            self.ecs.update_entity_name(
                                selected_id,
                                entity_name.clone(),
                            );
                            self.update_entity(selected_id);
                            self.print_to_terminal("Entity renamed successfully.");
                            self.new_entity_name.clear();
                            self.show_entity_rename_popup = false;

                        } else {
                            self.print_to_terminal("No entity selected.");
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        self.new_entity_name.clear();
                        self.show_entity_rename_popup = false;
                    }
                });

            });
    }


}