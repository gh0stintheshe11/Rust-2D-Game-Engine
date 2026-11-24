use crate::menu_bar::MenuBar;
use crate::panels::files::FileSystem;
use crate::panels::hierarchy::SceneHierarchy;
use crate::panels::inspector::Inspector;
use crate::state::GuiState;
use eframe::egui;
use rust_2d_game_engine::logger::{ConsoleMessage, ConsoleMessageType, LOGGER};
use rust_2d_game_engine::scripting::check_syntax;
use rust_2d_game_engine::{
    audio::AudioEngine, input::InputHandler, physics::PhysicsEngine, render::RenderEngine,
    runtime::GameRuntime, scene::SceneManager,
};
use std::fs;
use std::path::PathBuf;

mod console;
mod exit;
mod script_editor;
mod viewport;

pub struct EditorApp {
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
    // LOGGER revision console_messages was copied at
    console_revision: Option<u64>,
    selected_log_level: ConsoleMessageType,

    game_runtime: GameRuntime,

    editor_content: String,
    current_edited_file: Option<PathBuf>,
    editor_dirty: bool,
    // Latest Lua parse error of the editor buffer, if any
    syntax_error: Option<String>,
    show_api_palette: bool,
    // Set once the user has confirmed exiting; lets the close request through
    allow_close: bool,
    // Entity being moved in the viewport: (entity id, world-space grab offset)
    viewport_drag: Option<(uuid::Uuid, (f32, f32))>,
}

impl EditorApp {
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
        let input_handler = InputHandler::new();

        // Create GameRuntime with all required components
        let game_runtime = GameRuntime::new(
            SceneManager::new(),
            PhysicsEngine::new(),
            RenderEngine::new(),
            InputHandler::new(),
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
            console_revision: None,
            selected_log_level: ConsoleMessageType::Info,
            game_runtime,
            editor_content: String::new(),
            current_edited_file: None,
            editor_dirty: false,
            syntax_error: None,
            show_api_palette: false,
            allow_close: false,
            viewport_drag: None,
        }
    }

    fn show_windows(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.content_rect();
        let spacing = 4.0;
        let min_side_panel_width = 200.0;

        // Scene undo/redo shortcuts, only when no text field is focused
        // (focused TextEdits have their own Ctrl+Z handling)
        if ctx.memory(|m| m.focused().is_none()) {
            let undo_pressed = ctx.input_mut(|i| {
                i.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::CTRL,
                    egui::Key::Z,
                ))
            });
            let redo_pressed = ctx.input_mut(|i| {
                i.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::CTRL,
                    egui::Key::Y,
                )) || i.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
                    egui::Key::Z,
                ))
            });
            if undo_pressed {
                crate::panels::hierarchy::utils::perform_undo(&mut self.gui_state);
            }
            if redo_pressed {
                crate::panels::hierarchy::utils::perform_redo(&mut self.gui_state);
            }
        }

        // Another panel (hierarchy, inspector, files) asked to open a script
        if let Some(path) = self.gui_state.open_script_request.take() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    // Save the previous file before switching buffers
                    self.save_editor_if_dirty();
                    self.syntax_error = check_syntax(&content);
                    self.editor_content = content;
                    self.current_edited_file = Some(path);
                    self.editor_dirty = false;
                    self.show_editor = true;
                }
                Err(e) => LOGGER.error(format!("Failed to open {}: {}", path.display(), e)),
            }
        }

        // Another panel asked to insert a snippet at the editor cursor
        if let Some(snippet) = self.gui_state.script_insert_request.take() {
            self.insert_into_editor(ctx, &snippet);
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
                                        self.syntax_error = check_syntax(&content);
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
                            self.show_console_panel(ui);
                        });
                }

                // Center panel (Game view/Editor) should come after all other panels
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(2, 2)))
                    .show(ui, |ui| {
                        self.show_center_panel(ctx, ui);
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

    fn set_theme(&mut self, ctx: &egui::Context) {
        ctx.set_theme(if self.gui_state.dark_mode {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
    }
}

impl eframe::App for EditorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Background fill for the whole app area
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, self.get_background_color());

        self.show_windows(&ctx);
        self.handle_exit_flow(&ctx);

        let revision = LOGGER.revision();
        if self.console_revision != Some(revision) {
            self.console_messages = LOGGER.get_console_messages();
            self.console_revision = Some(revision);
        }
    }
}
