use crate::{
    physics_engine::PhysicsEngine,
    render_engine::RenderEngine,
    input_handler::InputHandler,
    audio_engine::AudioEngine,
    ecs::SceneManager,
    game_runtime::{GameRuntime, RuntimeState}
};
use crate::gui::gui_state::GuiState;
use crate::gui::menu_bar::MenuBar;
use crate::gui::scene_hierarchy::SceneHierarchy;
use crate::gui::file_system::FileSystem;
use crate::gui::inspector::Inspector;
use eframe::egui;
use std::fs;
use std::path::PathBuf;

struct ConsoleMessage {
    text: String,
    timestamp: chrono::DateTime<chrono::Local>,
    message_type: ConsoleMessageType,
}

enum ConsoleMessageType {
    Info,
    Warning,
    Error,
    Debug,
}

impl ConsoleMessage {
    fn new(text: String, message_type: ConsoleMessageType) -> Self {
        Self {
            text,
            timestamp: chrono::Local::now(),
            message_type,
        }
    }
}

const MAX_CONSOLE_MESSAGES: usize = 1000; // Adjust this number as needed

pub struct EngineGui {
    // Window States
    show_editor: bool,
    show_debug: bool,

    // Windows
    pub scene_hierarchy: SceneHierarchy,
    pub file_system: FileSystem,
    pub inspector: Inspector,
    pub menu_bar: MenuBar,

    // GUI settings
    pub gui_state: GuiState,

    // Add render engine
    render_engine: RenderEngine,

    // Add input handler
    input_handler: InputHandler,

    console_messages: Vec<ConsoleMessage>,

    game_runtime: GameRuntime,

    editor_content: String,
    current_edited_file: Option<PathBuf>,
}

impl EngineGui {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gui_state = GuiState::new();
        let render_engine = RenderEngine::new();
        let input_handler = InputHandler::new();
        
        // Create GameRuntime with all required components
        let game_runtime = GameRuntime::new(
            SceneManager::new(),
            PhysicsEngine::new(),
            render_engine.clone(), // We'll need to implement Clone for RenderEngine
            input_handler.clone(), // We'll need to implement Clone for InputHandler
            AudioEngine::new(),
            60  // target fps
        );

        Self {
            show_editor: false,
            show_debug: false,
            scene_hierarchy: SceneHierarchy::new(),
            file_system: FileSystem::new(),
            inspector: Inspector::new(),
            menu_bar: MenuBar::new(),
            gui_state,
            render_engine,
            input_handler,
            console_messages: Vec::new(),
            game_runtime,
            editor_content: String::new(),
            current_edited_file: None,
        }
    }

    fn show_windows(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.available_rect();
        let spacing = 4.0;
        let min_side_panel_width = 200.0;

        // Frame color
        let default_fill = self.get_background_color();

        self.set_theme(ctx);

        let main_window_frame = egui::Frame {
            inner_margin: egui::Margin::ZERO,
            outer_margin: egui::Margin::ZERO,
            rounding: egui::Rounding::ZERO,
            shadow: eframe::epaint::Shadow::NONE,
            fill: egui::Color32::TRANSPARENT,
            stroke: egui::Stroke::NONE,
        };

        // Viewport (Center)
        egui::Window::new("Main Window")
            .frame(main_window_frame)
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(spacing, spacing))
            .resizable(false)
            .collapsible(false)
            .movable(false)
            .title_bar(false)
            .fixed_size([
                screen_rect.width() - 2.0 * spacing,
                screen_rect.height() - 2.0 * spacing,
            ])
            .show(ctx, |ui| {
                // Menu bar at top
                ui.horizontal(|ui| {
                    self.menu_bar.show(ctx, ui, &mut self.gui_state);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.selectable_label(self.show_editor, "📝 Editor").clicked() {
                            self.show_editor = true;
                        }
                        if ui.selectable_label(!self.show_editor, "🎮 Viewer").clicked() {
                            self.show_editor = false;
                            if let Some(path) = &self.current_edited_file {
                                if let Err(err) = fs::write(path, &self.editor_content) {
                                    self.log_error(format!("Failed to save file: {}", err));
                                }
                            }
                        }
                    });
                });

                // Only add separator if any panel is visible
                if self.gui_state.show_hierarchy_filesystem || self.gui_state.show_inspector || self.gui_state.show_console {
                    ui.separator();
                }

                // Main content area with resizable panels
                let available_rect = ui.available_rect_before_wrap();

                // Left panel (Scene/Files)
                if self.gui_state.show_hierarchy_filesystem {
                    egui::SidePanel::left("left_panel")
                        .resizable(true)
                        .min_width(min_side_panel_width)
                        .max_width(available_rect.width() * 0.4)
                        .frame(egui::Frame {
                            inner_margin: egui::Margin::ZERO,
                            outer_margin: egui::Margin::ZERO,
                            rounding: egui::Rounding::ZERO,
                            shadow: eframe::epaint::Shadow::NONE,
                            fill: egui::Color32::TRANSPARENT,
                            stroke: egui::Stroke::NONE,
                        })
                        .show_inside(ui, |ui| {
                            // Use vertical layout to split the panel
                            egui::TopBottomPanel::top("scene_panel")
                                .resizable(true)
                                .min_height(200.0)
                                .max_height(ui.available_height() * 0.75)
                                .default_height(ui.available_height() * 0.5)
                                .frame(egui::Frame {
                                    inner_margin: egui::Margin::ZERO,
                                    outer_margin: egui::Margin::ZERO,
                                    rounding: egui::Rounding::ZERO,
                                    shadow: eframe::epaint::Shadow::NONE,
                                    fill: egui::Color32::TRANSPARENT,
                                    stroke: egui::Stroke::NONE,
                                })
                                .show_inside(ui, |ui| {
                                    self.scene_hierarchy.show(ctx, ui, &mut self.gui_state);
                                });

                            // Add the file system view in the bottom part
                            egui::CentralPanel::default()
                                .frame(egui::Frame {
                                    inner_margin: egui::Margin::ZERO,
                                    outer_margin: egui::Margin::ZERO,
                                    rounding: egui::Rounding::ZERO,
                                    shadow: eframe::epaint::Shadow::NONE,
                                    fill: egui::Color32::TRANSPARENT,
                                    stroke: egui::Stroke::NONE,
                                })
                                .show_inside(ui, |ui| {
                                    if let Some((path, content)) = self.file_system.show(ctx, ui, &mut self.gui_state) {
                                        self.editor_content = content;
                                        self.current_edited_file = Some(path);
                                    }
                                });
                        });
                }

                // Right panel (Inspector)
                if self.gui_state.show_inspector {

                    let inspector_margin = egui::Margin {
                        left: 6.0,
                        right: 4.0,
                        top: 0.0,
                        bottom: 4.0,
                    };
                    
                    egui::SidePanel::right("right_panel")
                        .resizable(true)
                        .min_width(min_side_panel_width)
                        .max_width(available_rect.width() * 0.4)
                        .frame(egui::Frame {
                            inner_margin: egui::Margin::ZERO,
                            outer_margin: inspector_margin,
                            rounding: egui::Rounding::ZERO,
                            shadow: eframe::epaint::Shadow::NONE,
                            fill: egui::Color32::TRANSPARENT,
                            stroke: egui::Stroke::NONE,
                        })
                        .show_inside(ui, |ui| {
                            ui.heading("Inspector");
                            ui.separator();
                            self.inspector.show(ctx, ui, &mut self.gui_state);
                        });
                }

                // Bottom panel (Console)
                if self.gui_state.show_console {
                    egui::TopBottomPanel::bottom("console_panel")
                        .resizable(true)
                        .min_height(100.0)
                        .default_height(200.0)
                        .max_height(ui.available_height() * 0.5)
                        .frame(egui::Frame::none()
                            .inner_margin(egui::Margin::symmetric(6.0, 8.0)))
                        .show_inside(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui.selectable_label(!self.show_debug, "💬 Output").clicked() {
                                    self.show_debug = false;
                                }
                                if ui.selectable_label(self.show_debug, "🛠 Debug").clicked() {
                                    self.show_debug = true;
                                }
                            });
                            ui.separator();
                            if self.show_debug {
                                ui.label("Debug info will go here");
                            } else {
                                egui::ScrollArea::vertical()
                                    .stick_to_bottom(true)
                                    .show_viewport(ui, |ui, _| {
                                        for message in &self.console_messages {
                                            let time_str = message.timestamp.format("%H:%M:%S").to_string();

                                            let (prefix, color) = match message.message_type {
                                                ConsoleMessageType::Info => ("ℹ", egui::Color32::LIGHT_BLUE),
                                                ConsoleMessageType::Warning => ("⚠", egui::Color32::YELLOW),
                                                ConsoleMessageType::Error => ("❌", egui::Color32::RED),
                                                ConsoleMessageType::Debug => ("🔧", egui::Color32::GRAY),
                                            };

                                            ui.horizontal(|ui| {
                                                ui.label(format!("[{}]", time_str));
                                                ui.colored_label(color, prefix);
                                                ui.label(&message.text);
                                            });
                                        }
                                    });
                            }
                        });
                }

                // Center panel (Game view/Editor) should come after all other panels
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(2.0, 2.0)))
                    .show_inside(ui, |ui| {
                        let content_rect = ui.available_rect_before_wrap();

                        // First fill the background
                        if self.show_editor {
                            ui.painter().rect_filled(
                                content_rect,
                                0.0,
                                egui::Color32::from_gray(40),
                            );

                            // Just show the editor, no file system here
                            let response = ui.add_sized(
                                content_rect.size(),
                                egui::TextEdit::multiline(&mut self.editor_content)
                                    .code_editor()
                                    .desired_width(f32::INFINITY),
                            );

                            // If editor content changed, save it
                            if response.changed() {
                                if let Some(path) = &self.current_edited_file {
                                    if let Err(err) = fs::write(path, &self.editor_content) {
                                        self.log_error(format!("Failed to save file: {}", err));
                                    }
                                }
                            }
                        } else {
                            // Render the game view first
                            self.render_scene(ui);

                            // Get viewport rect for input handling
                            let viewport_rect = ui.max_rect();

                            // Game control buttons floating on top
                            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.add_space((ui.available_width() - 170.0) * 0.5);
                                    
                                    // Check if a project is loaded
                                    if !self.gui_state.load_project {
                                        // No project loaded - show disabled buttons or message
                                        ui.add_enabled(false, egui::Button::new("▶ Play"));
                                        ui.add_enabled(false, egui::Button::new("⏸ Pause"));
                                        ui.add_enabled(false, egui::Button::new("⏹ Reset"));
                                        return;
                                    }

                                    // Project is loaded - show normal controls
                                    match self.game_runtime.get_state() {
                                        RuntimeState::Stopped => {
                                            if ui.button("▶ Play").clicked() {
                                                match self.game_runtime.run() {
                                                    Ok(_) => {
                                                        self.game_runtime.set_state(RuntimeState::Playing);
                                                        self.log_info("Game started successfully");
                                                    }
                                                    Err(error) => {
                                                        self.game_runtime.set_state(RuntimeState::Stopped);
                                                        self.log_error(format!("Failed to start game: {}", error));
                                                    }
                                                }
                                            }
                                        }
                                        RuntimeState::Playing => {
                                            if ui.button("⏸ Pause").clicked() {
                                                self.game_runtime.set_state(RuntimeState::Paused);
                                            }
                                        }
                                        RuntimeState::Paused => {
                                            if ui.button("▶ Resume").clicked() {
                                                self.game_runtime.set_state(RuntimeState::Playing);
                                                self.game_runtime.run().unwrap();
                                            }
                                        }
                                    }
                                    
                                    if ui.button("⏹ Reset").clicked() {
                                        self.game_runtime.reset();
                                    }
                                });
                            });

                            // Camera reset button in bottom right
                            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                                ui.add_space(4.0);  // Bottom margin
                                ui.horizontal(|ui| {
                                    ui.add_space(2.0);  // Right margin
                                    let button = ui.add_sized(
                                        [20.0, 20.0],  // Fixed size of 24x24 pixels
                                        egui::Button::new("🔄")
                                    );
                                    if button.clicked() {
                                        self.render_engine.camera.reset();
                                    }
                                    button.on_hover_text("Reset Camera");  // Tooltip text
                                });
                            });

                            // Update input handler
                            ui.ctx().input(|input| {
                                self.input_handler.handle_input(input);
                            });

                            // Only handle game input if cursor is in viewport
                            if let Some(cursor_pos) = ui.ctx().input(|i| i.pointer.hover_pos()) {
                                if viewport_rect.contains(cursor_pos) {
                                    // Handle camera pan with mouse drag
                                    if self.input_handler.is_mouse_button_pressed(egui::PointerButton::Secondary) || 
                                       (self.input_handler.is_mouse_button_pressed(egui::PointerButton::Primary) && 
                                        ui.ctx().input(|i| i.modifiers.alt)) {
                                        ui.ctx().input(|i| {
                                            let delta = i.pointer.delta();
                                            self.render_engine.camera.move_by(-delta.x, -delta.y);
                                        });
                                    }

                                    // Handle zoom with mouse wheel
                                    if let Some(scroll_delta) = self.input_handler.get_scroll_delta() {
                                        let zoom_factor = if scroll_delta.y > 0.0 { 1.1 } else { 0.9 };
                                        self.render_engine.camera.zoom_by(zoom_factor);
                                    }
                                }
                            }

                            // Debug overlay in the bottom-left of the game view
                            if self.gui_state.show_debug_overlay {
                                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                                    ui.add_space(38.0);  // Bottom margin
                                    ui.horizontal(|ui| {
                                        ui.add_space(4.0);  // Left margin
                                        ui.vertical(|ui| {
                                            let white = egui::Color32::WHITE;
                                            
                                            // Cursor position
                                            if let Some(cursor_pos) = ui.ctx().input(|i| i.pointer.hover_pos()) {
                                                ui.colored_label(white, format!("Cursor: ({:.1}, {:.1})", cursor_pos.x, cursor_pos.y));
                                            } else {
                                                ui.colored_label(white, "Cursor: Outside");
                                            }

                                            // Active inputs
                                            let all_inputs = self.input_handler.get_all_active_inputs();
                                            let keys_str = if all_inputs.is_empty() {
                                                "None".to_string()
                                            } else {
                                                all_inputs.join(", ")
                                            };
                                            ui.colored_label(white, format!("Keys: {}", keys_str));

                                            // Viewport size
                                            ui.colored_label(white, format!("Viewport: {:.0}x{:.0}", 
                                                viewport_rect.width(), viewport_rect.height()));
                                        });
                                    });
                                });
                            }
                        }
                    });
            });
    }

    fn get_background_color(&self) -> egui::Color32 {
        if self.gui_state.dark_mode {
            egui::Color32::from_gray(30) // Dark gray
        } else {
            egui::Color32::from_gray(240) // Light gray
        }
    }

    fn render_scene(&mut self, ui: &mut egui::Ui) {
        let content_rect = ui.available_rect_before_wrap();
        
        // Update render engine with the full viewport dimensions first
        self.render_engine.update_viewport_size(
            content_rect.width(),
            content_rect.height()
        );

        // Draw grid and game content using the full viewport area
        let grid_lines = self.render_engine.get_grid_lines();
        for (start, end) in grid_lines {
            ui.painter().line_segment(
                [
                    egui::pos2(content_rect.min.x + start.0, content_rect.min.y + start.1),
                    egui::pos2(content_rect.min.x + end.0, content_rect.min.y + end.1)
                ],
                egui::Stroke::new(0.5, egui::Color32::from_gray(60))
            );
        }

        // Render game content
        if let Some(scene_manager) = &self.gui_state.scene_manager {
            if let Some(active_scene) = scene_manager.get_active_scene() {
                // First render all game objects
                let render_queue = self.render_engine.render(active_scene);
                
                for (texture_id, pos, size, _layer) in render_queue {
                    if let Some(texture_info) = self.render_engine.texture_cache.get(&texture_id) {
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(
                                content_rect.min.x + pos.0,
                                content_rect.min.y + pos.1,
                            ),
                            egui::vec2(size.0, size.1),
                        );

                        let texture = ui.ctx().load_texture(
                            format!("texture_{}", texture_id),
                            egui::ColorImage::from_rgba_unmultiplied(
                                [texture_info.dimensions.0 as usize, texture_info.dimensions.1 as usize],
                                &texture_info.data,
                            ),
                            Default::default()
                        );

                        ui.painter().image(
                            texture.id(),
                            rect,
                            egui::Rect::from_min_max(
                                egui::pos2(0.0, 0.0),
                                egui::pos2(1.0, 1.0),
                            ),
                            egui::Color32::WHITE,
                        );
                    }
                }

                // Then draw the game camera bounds
                let camera_lines = self.render_engine.get_game_camera_bounds(active_scene);
                for (start, end) in camera_lines {
                    ui.painter().line_segment(
                        [
                            egui::pos2(content_rect.min.x + start.0, content_rect.min.y + start.1),
                            egui::pos2(content_rect.min.x + end.0, content_rect.min.y + end.1)
                        ],
                        egui::Stroke::new(2.0, egui::Color32::RED)
                    );
                }
            }
        }
    }

    fn set_theme(&mut self, ctx: &egui::Context) {
        let visuals = if self.gui_state.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        // avoid repaint everytime
        if ctx.style().visuals != visuals {
            ctx.set_visuals(visuals);
        }
    }

    fn add_message(&mut self, message: ConsoleMessage) {
        self.console_messages.push(message);
        // Remove oldest messages if we exceed the limit
        if self.console_messages.len() > MAX_CONSOLE_MESSAGES {
            let excess = self.console_messages.len() - MAX_CONSOLE_MESSAGES;
            self.console_messages.drain(0..excess);
        }
    }

    pub fn log_info(&mut self, message: impl Into<String>) {
        self.add_message(ConsoleMessage::new(
            message.into(),
            ConsoleMessageType::Info,
        ));
    }

    pub fn log_warning(&mut self, message: impl Into<String>) {
        self.add_message(ConsoleMessage::new(
            message.into(),
            ConsoleMessageType::Warning,
        ));
    }

    pub fn log_error(&mut self, message: impl Into<String>) {
        self.add_message(ConsoleMessage::new(
            message.into(),
            ConsoleMessageType::Error,
        ));
    }

    pub fn log_debug(&mut self, message: impl Into<String>) {
        self.add_message(ConsoleMessage::new(
            message.into(),
            ConsoleMessageType::Debug,
        ));
    }
}

impl eframe::App for EngineGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame {
                inner_margin: egui::Margin::ZERO,
                outer_margin: egui::Margin::ZERO,
                rounding: egui::Rounding::ZERO,
                shadow: eframe::epaint::Shadow::NONE,
                fill: self.get_background_color(),
                stroke: egui::Stroke::NONE,
            })
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                self.show_windows(ctx);
            });
    }
}
