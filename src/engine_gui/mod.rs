use crate::gui::file_system::FileSystem;
use crate::gui::gui_state::{ExitRequest, GuiState};
use crate::gui::inspector::Inspector;
use crate::gui::menu_bar::MenuBar;
use crate::gui::scene_hierarchy::SceneHierarchy;
use crate::logger::{ConsoleMessage, ConsoleMessageType, LOGGER};
use crate::{
    audio_engine::AudioEngine,
    ecs::SceneManager,
    game_runtime::{GameRuntime, RuntimeState},
    input_handler::{InputContext, InputHandler},
    physics_engine::PhysicsEngine,
    render_engine::RenderEngine,
};
use eframe::egui;
use std::fs;
use std::path::PathBuf;

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
    selected_log_level: ConsoleMessageType,

    game_runtime: GameRuntime,

    editor_content: String,
    current_edited_file: Option<PathBuf>,
    editor_dirty: bool,
    // Set once the user has confirmed exiting; lets the close request through
    allow_close: bool,
    // Entity being moved in the viewport: (entity id, world-space grab offset)
    viewport_drag: Option<(uuid::Uuid, (f32, f32))>,
}

impl EngineGui {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Remove shadows for all popups/windows, for both themes (done once
        // here instead of mutating visuals every frame)
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            let mut visuals = theme.default_visuals();
            visuals.popup_shadow = egui::epaint::Shadow::NONE;
            visuals.window_shadow = egui::epaint::Shadow::NONE;
            cc.egui_ctx.set_visuals_of(theme, visuals);
        }

        let gui_state = GuiState::new();
        let render_engine = RenderEngine::new();
        let mut input_handler = InputHandler::new();
        input_handler.set_context(InputContext::EngineUI); // Make sure we start in EngineUI mode

        // Create GameRuntime with all required components
        let game_runtime = GameRuntime::new(
            SceneManager::new(),
            PhysicsEngine::new(),
            render_engine.clone(), // We'll need to implement Clone for RenderEngine
            input_handler.clone(), // We'll need to implement Clone for InputHandler
            AudioEngine::new(),
            60, // target fps
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
            selected_log_level: ConsoleMessageType::Info,
            game_runtime,
            editor_content: String::new(),
            current_edited_file: None,
            editor_dirty: false,
            allow_close: false,
            viewport_drag: None,
        }
    }

    /// Write the editor buffer to its file if it has unsaved changes.
    fn save_editor_if_dirty(&mut self) {
        if !self.editor_dirty {
            return;
        }
        if let Some(path) = &self.current_edited_file {
            match fs::write(path, &self.editor_content) {
                Ok(_) => {
                    self.editor_dirty = false;
                    LOGGER.info(format!("Saved {}", path.display()));
                }
                Err(err) => LOGGER.error(format!("Failed to save file: {}", err)),
            }
        }
    }

    fn show_windows(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.content_rect();
        let spacing = 4.0;
        let min_side_panel_width = 200.0;

        // Another panel (hierarchy, inspector, files) asked to open a script
        if let Some(path) = self.gui_state.open_script_request.take() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    // Save the previous file before switching buffers
                    self.save_editor_if_dirty();
                    self.editor_content = content;
                    self.current_edited_file = Some(path);
                    self.editor_dirty = false;
                    self.show_editor = true;
                }
                Err(e) => LOGGER.error(format!("Failed to open {}: {}", path.display(), e)),
            }
        }

        // Frame color
        let _default_fill = self.get_background_color();

        self.set_theme(ctx);

        let main_window_frame = egui::Frame {
            inner_margin: egui::Margin::ZERO,
            outer_margin: egui::Margin::ZERO,
            corner_radius: egui::CornerRadius::ZERO,
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
                        if ui
                            .selectable_label(!self.show_editor, "🎮 Viewer")
                            .clicked()
                        {
                            self.show_editor = false;
                            self.save_editor_if_dirty();
                        }
                    });
                });

                // Only add separator if any panel is visible
                if self.gui_state.show_hierarchy_filesystem
                    || self.gui_state.show_inspector
                    || self.gui_state.show_console
                {
                    ui.separator();
                }

                // Main content area with resizable panels
                let available_rect = ui.available_rect_before_wrap();

                // Left panel (Scene/Files)
                if self.gui_state.show_hierarchy_filesystem {
                    egui::Panel::left("left_panel")
                        .resizable(true)
                        .min_size(min_side_panel_width)
                        .max_size(available_rect.width() * 0.4)
                        .frame(egui::Frame {
                            inner_margin: egui::Margin::ZERO,
                            outer_margin: egui::Margin::ZERO,
                            corner_radius: egui::CornerRadius::ZERO,
                            shadow: eframe::epaint::Shadow::NONE,
                            fill: egui::Color32::TRANSPARENT,
                            stroke: egui::Stroke::NONE,
                        })
                        .show(ui, |ui| {
                            // Use vertical layout to split the panel
                            egui::Panel::top("scene_panel")
                                .resizable(true)
                                .min_size(200.0)
                                .max_size(ui.available_height() * 0.75)
                                .default_size(ui.available_height() * 0.5)
                                .frame(egui::Frame {
                                    inner_margin: egui::Margin::ZERO,
                                    outer_margin: egui::Margin::ZERO,
                                    corner_radius: egui::CornerRadius::ZERO,
                                    shadow: eframe::epaint::Shadow::NONE,
                                    fill: egui::Color32::TRANSPARENT,
                                    stroke: egui::Stroke::NONE,
                                })
                                .show(ui, |ui| {
                                    self.scene_hierarchy.show(ctx, ui, &mut self.gui_state);
                                });

                            // Add the file system view in the bottom part
                            egui::CentralPanel::default()
                                .frame(egui::Frame {
                                    inner_margin: egui::Margin::ZERO,
                                    outer_margin: egui::Margin::ZERO,
                                    corner_radius: egui::CornerRadius::ZERO,
                                    shadow: eframe::epaint::Shadow::NONE,
                                    fill: egui::Color32::TRANSPARENT,
                                    stroke: egui::Stroke::NONE,
                                })
                                .show(ui, |ui| {
                                    if let Some((path, content)) =
                                        self.file_system.show(ctx, ui, &mut self.gui_state)
                                    {
                                        // Save the previous file before switching buffers
                                        self.save_editor_if_dirty();
                                        self.editor_content = content;
                                        self.current_edited_file = Some(path);
                                        self.editor_dirty = false;
                                        self.show_editor = true;
                                    }
                                });
                        });
                }

                // Right panel (Inspector)
                if self.gui_state.show_inspector {
                    let inspector_margin = egui::Margin {
                        left: 6,
                        right: 4,
                        top: 0,
                        bottom: 4,
                    };

                    egui::Panel::right("right_panel")
                        .resizable(true)
                        .min_size(min_side_panel_width)
                        .max_size(available_rect.width() * 0.4)
                        .frame(egui::Frame {
                            inner_margin: egui::Margin::ZERO,
                            outer_margin: inspector_margin,
                            corner_radius: egui::CornerRadius::ZERO,
                            shadow: eframe::epaint::Shadow::NONE,
                            fill: egui::Color32::TRANSPARENT,
                            stroke: egui::Stroke::NONE,
                        })
                        .show(ui, |ui| {
                            ui.heading("Inspector");
                            ui.separator();
                            self.inspector.show(ctx, ui, &mut self.gui_state);
                        });
                }

                // Bottom panel (Console)
                if self.gui_state.show_console {
                    egui::Panel::bottom("console_panel")
                        .resizable(true)
                        .min_size(100.0)
                        .default_size(200.0)
                        .max_size(ui.available_height() * 0.5)
                        .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(6, 8)))
                        .show(ui, |ui| {
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
                                self.show_console_messages(
                                    ui,
                                    &self.console_messages,
                                    ConsoleMessageType::Debug,
                                );
                            } else {
                                egui::ComboBox::from_label("Log Level")
                                    .selected_text(format!("{:?}", self.selected_log_level))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.selected_log_level,
                                            ConsoleMessageType::Info,
                                            "Info",
                                        );
                                        ui.selectable_value(
                                            &mut self.selected_log_level,
                                            ConsoleMessageType::Warning,
                                            "Warning",
                                        );
                                        ui.selectable_value(
                                            &mut self.selected_log_level,
                                            ConsoleMessageType::Error,
                                            "Error",
                                        );
                                    });
                                self.show_console_messages(
                                    ui,
                                    &self.console_messages,
                                    self.selected_log_level.clone(),
                                );
                            }
                        });
                }

                // Center panel (Game view/Editor) should come after all other panels
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(2, 2)))
                    .show(ui, |ui| {
                        let content_rect = ui.available_rect_before_wrap();

                        // First fill the background
                        if self.show_editor {
                            ui.painter().rect_filled(
                                content_rect,
                                0.0,
                                egui::Color32::from_gray(40),
                            );

                            // Ctrl+S saves the current file
                            let save_requested = ui.input_mut(|i| {
                                i.consume_shortcut(&egui::KeyboardShortcut::new(
                                    egui::Modifiers::CTRL,
                                    egui::Key::S,
                                ))
                            });
                            if save_requested {
                                self.save_editor_if_dirty();
                            }

                            // Filename + unsaved indicator
                            ui.horizontal(|ui| {
                                let label = match &self.current_edited_file {
                                    Some(path) => format!(
                                        "📄 {}{}",
                                        path.file_name()
                                            .map(|n| n.to_string_lossy().into_owned())
                                            .unwrap_or_default(),
                                        if self.editor_dirty { " ●" } else { "" }
                                    ),
                                    None => "No file open — click a script in the Files panel \
                                             or in the scene hierarchy"
                                        .to_string(),
                                };
                                ui.label(label);
                                if self.editor_dirty {
                                    ui.weak("(Ctrl+S to save)");
                                }
                            });

                            let theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(
                                ui.ctx(),
                                ui.style(),
                            );

                            let mut layouter = |ui: &egui::Ui,
                                                text: &dyn egui::TextBuffer,
                                                wrap_width: f32| {
                                let mut layout_job = egui_extras::syntax_highlighting::highlight(
                                    ui.ctx(),
                                    ui.style(),
                                    &theme,
                                    text.as_str(),
                                    "lua",
                                );
                                layout_job.wrap.max_width = wrap_width;
                                ui.fonts_mut(|f| f.layout_job(layout_job))
                            };

                            egui::ScrollArea::both()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    // Just show the editor, no file system here
                                    let response = ui.add_sized(
                                        content_rect.size(),
                                        egui::TextEdit::multiline(&mut self.editor_content)
                                            .code_editor()
                                            .lock_focus(true)
                                            .desired_width(f32::INFINITY)
                                            .layouter(&mut layouter),
                                    );

                                    // Mark dirty on edits; the buffer is written on
                                    // Ctrl+S, focus loss, or when switching files/views
                                    if response.changed() {
                                        self.editor_dirty = true;
                                    }
                                    if response.lost_focus() {
                                        self.save_editor_if_dirty();
                                    }
                                });
                        } else {
                            // Render only the viewport content when play in the GUI.
                            // Paused/Ended still route to the runtime so the freeze
                            // frame of the running game stays visible.
                            let runtime_state = self.game_runtime.get_state();
                            if matches!(
                                runtime_state,
                                RuntimeState::Playing | RuntimeState::Paused | RuntimeState::Ended
                            ) {
                                // sync camera to runtime
                                let position = self.render_engine.camera.position;
                                let zoom = self.render_engine.camera.zoom;
                                self.game_runtime.set_camera_state(position, zoom);

                                let game_view_rect = ui.available_rect_before_wrap();
                                self.game_runtime.update(ctx, ui, game_view_rect);

                                // Then draw the game camera bounds
                                if let Some(scene_manager) = &self.gui_state.scene_manager {
                                    if let Some(active_scene) = scene_manager.get_active_scene() {
                                        let camera_lines =
                                            self.render_engine.get_game_camera_bounds(active_scene);
                                        for (start, end) in camera_lines {
                                            ui.painter().line_segment(
                                                [
                                                    egui::pos2(
                                                        content_rect.min.x + start.0,
                                                        content_rect.min.y + start.1,
                                                    ),
                                                    egui::pos2(
                                                        content_rect.min.x + end.0,
                                                        content_rect.min.y + end.1,
                                                    ),
                                                ],
                                                egui::Stroke::new(2.0_f32, egui::Color32::RED),
                                            );
                                        }
                                    }
                                }
                            } else {
                                // Render the game view first
                                self.render_scene(ui);
                            }

                            // Get viewport rect for input handling
                            let viewport_rect = ui.max_rect();

                            // Game control buttons floating on top
                            ui.with_layout(
                                egui::Layout::top_down_justified(egui::Align::Center),
                                |ui| {
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
                                                    // Scripts are (re)loaded from disk on play
                                                    self.save_editor_if_dirty();
                                                    // Sync scene manager before starting
                                                    self.sync_scene_manager_to_runtime();

                                                    match self.game_runtime.run() {
                                                        Ok(_) => {
                                                            self.game_runtime
                                                                .set_state(RuntimeState::Playing);
                                                            LOGGER
                                                                .info("Game started successfully");
                                                        }
                                                        Err(error) => {
                                                            self.game_runtime
                                                                .set_state(RuntimeState::Stopped);
                                                            LOGGER.error(format!(
                                                                "Failed to start game: {}",
                                                                error
                                                            ));
                                                        }
                                                    }
                                                }
                                            }
                                            RuntimeState::Playing => {
                                                if ui.button("⏸ Pause").clicked() {
                                                    self.game_runtime
                                                        .set_state(RuntimeState::Paused);
                                                }
                                            }
                                            RuntimeState::Paused => {
                                                if ui.button("▶ Resume").clicked() {
                                                    // Just unpause; the physics world and scene state
                                                    // from before the pause are still loaded
                                                    self.game_runtime
                                                        .set_state(RuntimeState::Playing);
                                                }
                                            }
                                            RuntimeState::Ended => {
                                                // Game over: only Reset makes sense
                                                ui.add_enabled(
                                                    false,
                                                    egui::Button::new("⏹ Game Over"),
                                                )
                                                .on_disabled_hover_text(
                                                    "A script ended the game. Press Reset to restore the scene.",
                                                );
                                            }
                                        }

                                        if ui.button("⏹ Reset").clicked() {
                                            self.game_runtime.reset();
                                        }
                                    });
                                },
                            );

                            // Camera reset button in bottom right
                            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                                ui.add_space(4.0); // Bottom margin
                                ui.horizontal(|ui| {
                                    ui.add_space(2.0); // Right margin
                                    let button = ui.add_sized(
                                        [20.0, 20.0], // Fixed size of 24x24 pixels
                                        egui::Button::new("🔄"),
                                    );
                                    if button.clicked() {
                                        self.render_engine.camera.reset();
                                    }
                                    button.on_hover_text("Reset Camera"); // Tooltip text
                                });
                            });

                            // Handle input based on game state
                            ctx.input(|input| {
                                match self.game_runtime.get_state() {
                                    RuntimeState::Playing => {
                                        // When playing, route input directly to game runtime's input handler
                                        self.game_runtime.handle_input(input);
                                    }
                                    _ => {
                                        // When not playing, use engine UI input handler
                                        self.input_handler.handle_input(input);
                                    }
                                }
                            });

                            // Then handle editor viewport controls only when not playing
                            if self.game_runtime.get_state() != RuntimeState::Playing {
                                if let Some(cursor_pos) = ui.ctx().input(|i| i.pointer.hover_pos())
                                {
                                    if viewport_rect.contains(cursor_pos) {
                                        // Editor camera controls
                                        if self
                                            .input_handler
                                            .is_mouse_button_pressed(egui::PointerButton::Secondary)
                                            || (self.input_handler.is_mouse_button_pressed(
                                                egui::PointerButton::Primary,
                                            ) && ui.ctx().input(|i| i.modifiers.alt))
                                        {
                                            ui.ctx().input(|i| {
                                                let delta = i.pointer.delta();
                                                // Convert screen-space delta to world units
                                                let zoom = self
                                                    .render_engine
                                                    .camera
                                                    .zoom
                                                    .max(0.0001);
                                                self.render_engine
                                                    .camera
                                                    .move_by(-delta.x / zoom, -delta.y / zoom);
                                            });
                                        }

                                        // Editor zoom control: zoom towards the cursor
                                        if let Some(scroll_delta) =
                                            self.input_handler.get_scroll_delta()
                                        {
                                            let zoom_factor =
                                                if scroll_delta.y > 0.0 { 1.1 } else { 0.9 };

                                            let camera = &mut self.render_engine.camera;
                                            let old_zoom = camera.zoom.max(0.0001);
                                            let local = cursor_pos - viewport_rect.min;
                                            // World point under the cursor before zooming
                                            let world_x =
                                                local.x / old_zoom + camera.position.0;
                                            let world_y =
                                                local.y / old_zoom + camera.position.1;

                                            camera.zoom_by(zoom_factor);

                                            // Keep that world point under the cursor
                                            let new_zoom = camera.zoom.max(0.0001);
                                            camera.position.0 = world_x - local.x / new_zoom;
                                            camera.position.1 = world_y - local.y / new_zoom;
                                        }
                                    }
                                }
                            }

                            // Debug overlay in the bottom-left of the game view
                            if self.gui_state.show_debug_overlay {
                                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                                    ui.add_space(38.0); // Bottom margin
                                    ui.horizontal(|ui| {
                                        ui.add_space(4.0); // Left margin
                                        ui.vertical(|ui| {
                                            let white = egui::Color32::WHITE;

                                            // Cursor position
                                            if let Some(cursor_pos) =
                                                ui.ctx().input(|i| i.pointer.hover_pos())
                                            {
                                                ui.colored_label(
                                                    white,
                                                    format!(
                                                        "Cursor: ({:.1}, {:.1})",
                                                        cursor_pos.x, cursor_pos.y
                                                    ),
                                                );
                                            } else {
                                                ui.colored_label(white, "Cursor: Outside");
                                            }

                                            // Active inputs
                                            let all_inputs =
                                                self.input_handler.get_all_active_inputs();
                                            let keys_str = if all_inputs.is_empty() {
                                                "None".to_string()
                                            } else {
                                                all_inputs.join(", ")
                                            };
                                            ui.colored_label(white, format!("Keys: {}", keys_str));

                                            // Viewport size
                                            ui.colored_label(
                                                white,
                                                format!(
                                                    "Viewport: {:.0}x{:.0}",
                                                    viewport_rect.width(),
                                                    viewport_rect.height()
                                                ),
                                            );
                                        });
                                    });
                                });
                            }
                        }
                    });
            });
    }

    fn show_console_messages(
        &self,
        ui: &mut egui::Ui,
        console_messages: &Vec<ConsoleMessage>,
        selected_level: ConsoleMessageType,
    ) {
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show_viewport(ui, |ui, _| {
                for message in console_messages {
                    let should_show = if selected_level == ConsoleMessageType::Debug {
                        message.message_type == ConsoleMessageType::Debug
                    } else {
                        message.message_type >= selected_level
                            && message.message_type != ConsoleMessageType::Debug
                    };

                    if should_show {
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
                            ui.allocate_exact_size(
                                egui::Vec2::new(ui.available_width(), 0.0),
                                egui::Sense::hover(),
                            );
                        });
                    }
                }
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
        self.render_engine
            .update_viewport_size(content_rect.width(), content_rect.height());

        // Draw grid and game content using the full viewport area
        let grid_lines = self.render_engine.get_grid_lines();
        for (start, end) in grid_lines {
            ui.painter().line_segment(
                [
                    egui::pos2(content_rect.min.x + start.0, content_rect.min.y + start.1),
                    egui::pos2(content_rect.min.x + end.0, content_rect.min.y + end.1),
                ],
                egui::Stroke::new(0.5_f32, egui::Color32::from_gray(60)),
            );
        }

        // Render game content
        let Some(scene_manager) = &self.gui_state.scene_manager else {
            return;
        };
        let Some(active_scene) = scene_manager.get_active_scene() else {
            return;
        };
        let active_scene_id = active_scene.id;
        let render_queue = self.render_engine.render(active_scene);

        let selected_entity = match self.gui_state.scene_panel_selected_item {
            crate::gui::gui_state::ScenePanelSelectedItem::Entity(_, entity_id) => Some(entity_id),
            _ => None,
        };

        for entry in &render_queue {
            if let Some(texture) = self
                .render_engine
                .get_egui_texture(ui.ctx(), entry.texture_id)
            {
                let rect = egui::Rect::from_min_size(
                    egui::pos2(
                        content_rect.min.x + entry.screen_pos.0,
                        content_rect.min.y + entry.screen_pos.1,
                    ),
                    egui::vec2(entry.screen_size.0, entry.screen_size.1),
                );

                crate::render_engine::paint_sprite(
                    ui.painter(),
                    texture.id(),
                    rect,
                    entry.rotation,
                );

                // Selection highlight
                if selected_entity == Some(entry.entity_id) {
                    ui.painter().rect_stroke(
                        rect,
                        0.0,
                        egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 200, 0)),
                        egui::StrokeKind::Outside,
                    );
                }
            }
        }

        // Then draw the game camera bounds
        let Some(scene_manager) = &self.gui_state.scene_manager else {
            return;
        };
        if let Some(active_scene) = scene_manager.get_active_scene() {
            let camera_lines = self.render_engine.get_game_camera_bounds(active_scene);
            for (start, end) in camera_lines {
                ui.painter().line_segment(
                    [
                        egui::pos2(content_rect.min.x + start.0, content_rect.min.y + start.1),
                        egui::pos2(content_rect.min.x + end.0, content_rect.min.y + end.1),
                    ],
                    egui::Stroke::new(2.0_f32, egui::Color32::RED),
                );
            }
        }

        self.handle_viewport_interaction(ui, content_rect, active_scene_id, &render_queue);
    }

    /// Left-click selects the topmost entity under the cursor; left-drag
    /// moves it (camera pan stays on right-drag / Alt+drag).
    fn handle_viewport_interaction(
        &mut self,
        ui: &mut egui::Ui,
        content_rect: egui::Rect,
        active_scene_id: uuid::Uuid,
        render_queue: &[crate::render_engine::RenderQueueEntry],
    ) {
        let response = ui.interact(
            content_rect,
            ui.make_persistent_id("scene_viewport_interaction"),
            egui::Sense::click_and_drag(),
        );

        let alt_held = ui.ctx().input(|i| i.modifiers.alt);
        let zoom = self.render_engine.camera.zoom.max(0.0001);

        // Topmost entity under a screen position (queue is z-sorted low->high)
        let hit_test = |pointer: egui::Pos2| -> Option<uuid::Uuid> {
            render_queue.iter().rev().find_map(|entry| {
                let rect = egui::Rect::from_min_size(
                    egui::pos2(
                        content_rect.min.x + entry.screen_pos.0,
                        content_rect.min.y + entry.screen_pos.1,
                    ),
                    egui::vec2(entry.screen_size.0, entry.screen_size.1),
                );
                rect.contains(pointer).then_some(entry.entity_id)
            })
        };

        // Click: select (or clear the selection when clicking empty space)
        if response.clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                match hit_test(pointer) {
                    Some(entity_id) => {
                        self.gui_state.selected_item =
                            crate::gui::gui_state::SelectedItem::Entity(active_scene_id, entity_id);
                        self.gui_state.scene_panel_selected_item =
                            crate::gui::gui_state::ScenePanelSelectedItem::Entity(
                                active_scene_id,
                                entity_id,
                            );
                    }
                    None => {
                        self.gui_state.selected_item = crate::gui::gui_state::SelectedItem::None;
                        self.gui_state.scene_panel_selected_item =
                            crate::gui::gui_state::ScenePanelSelectedItem::None;
                    }
                }
            }
        }

        // Drag start: grab the entity under the cursor (Alt+drag pans instead)
        if response.drag_started_by(egui::PointerButton::Primary) && !alt_held {
            if let Some(pointer) = response.interact_pointer_pos() {
                if let Some(entity_id) = hit_test(pointer) {
                    // World position of the cursor and the entity's grab offset
                    let camera = &self.render_engine.camera;
                    let world_x = (pointer.x - content_rect.min.x) / zoom + camera.position.0;
                    let world_y = (pointer.y - content_rect.min.y) / zoom + camera.position.1;

                    if let Some(scene_manager) = &self.gui_state.scene_manager {
                        if let Some(scene) = scene_manager.get_scene(active_scene_id) {
                            if let Ok(entity) = scene.get_entity(entity_id) {
                                let offset = (world_x - entity.get_x(), world_y - entity.get_y());
                                self.viewport_drag = Some((entity_id, offset));
                                // Select what we grab
                                self.gui_state.selected_item =
                                    crate::gui::gui_state::SelectedItem::Entity(
                                        active_scene_id,
                                        entity_id,
                                    );
                                self.gui_state.scene_panel_selected_item =
                                    crate::gui::gui_state::ScenePanelSelectedItem::Entity(
                                        active_scene_id,
                                        entity_id,
                                    );
                            }
                        }
                    }
                }
            }
        }

        // Dragging: move the grabbed entity with the cursor
        if response.dragged_by(egui::PointerButton::Primary) && !alt_held {
            if let (Some((entity_id, offset)), Some(pointer)) =
                (self.viewport_drag, response.interact_pointer_pos())
            {
                let camera = &self.render_engine.camera;
                let world_x = (pointer.x - content_rect.min.x) / zoom + camera.position.0;
                let world_y = (pointer.y - content_rect.min.y) / zoom + camera.position.1;
                let new_x = world_x - offset.0;
                let new_y = world_y - offset.1;

                if let Some(scene_manager) = &mut self.gui_state.scene_manager {
                    if let Some(scene) = scene_manager.get_scene_mut(active_scene_id) {
                        if let Ok(entity) = scene.get_entity_mut(entity_id) {
                            let _ = entity.set_x(new_x);
                            let _ = entity.set_y(new_y);
                            // Keep the optional position Vector2 in sync
                            if let Ok(pos_attr) = entity.get_attribute_by_name("position") {
                                let pos_id = pos_attr.id;
                                let _ = entity.modify_attribute(
                                    pos_id,
                                    None,
                                    None,
                                    Some(crate::ecs::AttributeValue::Vector2(new_x, new_y)),
                                );
                            }
                        }
                    }
                }
            }
        }

        // Drag end: persist the move
        if response.drag_stopped_by(egui::PointerButton::Primary)
            && self.viewport_drag.take().is_some()
        {
            crate::gui::scene_hierarchy::utils::save_project(&self.gui_state);
        }
    }

    fn set_theme(&mut self, ctx: &egui::Context) {
        ctx.set_theme(if self.gui_state.dark_mode {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
    }

    fn sync_scene_manager_to_runtime(&mut self) {
        // Get the scene manager from GUI state
        if let Some(gui_scene_manager) = &self.gui_state.scene_manager {
            // Update the game runtime's scene manager
            self.game_runtime
                .set_scene_manager(gui_scene_manager.clone());
            println!(
                "Synced scene manager to runtime with {} scenes",
                self.game_runtime.get_scene_manager().list_scene().len()
            );
        }
    }

    /// Exit confirmation dialog + the actual exit path. Triggered by
    /// File > Exit or the native window close button.
    fn handle_exit_flow(&mut self, ctx: &egui::Context) {
        // Intercept the native close button so unsaved work gets a prompt
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_close
            && self.gui_state.load_project
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.gui_state.exit_request = ExitRequest::PromptOpen;
        }

        if self.gui_state.exit_request == ExitRequest::PromptOpen {
            // Nothing worth prompting about without an open project
            if !self.gui_state.load_project {
                self.gui_state.exit_request = ExitRequest::ExitWithoutSaving;
            } else {
                egui::Window::new("Exit")
                    .collapsible(false)
                    .resizable(false)
                    .order(egui::Order::Foreground)
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .show(ctx, |ui| {
                        ui.label("Save the project before exiting?");
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Save & Exit").clicked() {
                                self.gui_state.exit_request = ExitRequest::SaveAndExit;
                            }
                            if ui.button("Exit without saving").clicked() {
                                self.gui_state.exit_request = ExitRequest::ExitWithoutSaving;
                            }
                            if ui.button("Cancel").clicked() {
                                self.gui_state.exit_request = ExitRequest::None;
                            }
                        });
                    });
            }
        }

        match self.gui_state.exit_request {
            ExitRequest::SaveAndExit => {
                self.save_editor_if_dirty();
                crate::gui::scene_hierarchy::utils::save_project(&self.gui_state);
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ExitRequest::ExitWithoutSaving => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            _ => {}
        }
    }
}

impl eframe::App for EngineGui {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Background fill for the whole app area
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, self.get_background_color());

        self.show_windows(&ctx);
        self.handle_exit_flow(&ctx);

        self.console_messages = LOGGER.get_console_messages();
    }
}
