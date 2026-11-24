//! The editor application: window layout and the state tying the panels,
//! the script editor and the game runtime together.

use crate::menu_bar::MenuBar;
use crate::panels::files::FileSystem;
use crate::panels::hierarchy::SceneHierarchy;
use crate::panels::inspector::Inspector;
use crate::state::GuiState;
use eframe::egui;
use rust_2d_game_engine::logger::{ConsoleMessage, ConsoleMessageType, LOGGER};
use rust_2d_game_engine::{
    audio::AudioEngine, input::InputHandler, physics::PhysicsEngine, render::RenderEngine,
    runtime::GameRuntime, scene::SceneManager,
};
use std::path::PathBuf;

mod console;
mod exit;
mod script_editor;
mod viewport;

pub struct EditorApp {
    // Window States
    show_editor: bool,
    show_debug: bool,

    // Panels
    scene_hierarchy: SceneHierarchy,
    file_system: FileSystem,
    inspector: Inspector,
    menu_bar: MenuBar,

    gui_state: GuiState,
    /// The project generation the panels were last reset for
    seen_project_generation: u64,

    /// Renders the editing view of the scene (the game has its own)
    render_engine: RenderEngine,
    /// Drives the editor camera while the game isn't playing
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
    // Move the script editor's cursor to the top on the next frame
    reset_editor_cursor: bool,
    // Set once the user has confirmed exiting; lets the close request through
    allow_close: bool,
    // Entity being moved in the viewport: (entity id, world-space grab offset)
    viewport_drag: Option<(uuid::Uuid, (f32, f32))>,
}

impl EditorApp {
    pub fn new(ctx: &egui::Context) -> Self {
        // Remove shadows for all popups/windows, for both themes (done once
        // here instead of mutating visuals every frame)
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            let mut visuals = theme.default_visuals();
            visuals.popup_shadow = egui::epaint::Shadow::NONE;
            visuals.window_shadow = egui::epaint::Shadow::NONE;
            ctx.set_visuals_of(theme, visuals);
        }

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
            gui_state: GuiState::new(),
            seen_project_generation: 0,
            render_engine: RenderEngine::new(),
            input_handler: InputHandler::new(),
            console_messages: Vec::new(),
            console_revision: None,
            selected_log_level: ConsoleMessageType::Info,
            game_runtime,
            editor_content: String::new(),
            current_edited_file: None,
            editor_dirty: false,
            syntax_error: None,
            show_api_palette: false,
            reset_editor_cursor: false,
            allow_close: false,
            viewport_drag: None,
        }
    }

    /// One editor frame; `App::ui` and the headless tests call this.
    pub fn frame(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();

        // Background fill for the whole app area
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, self.get_background_color());

        self.show_windows(&ctx);
        self.handle_requests();
        self.handle_exit_flow(&ctx);

        let revision = LOGGER.revision();
        if self.console_revision != Some(revision) {
            self.console_messages = LOGGER.get_console_messages();
            self.console_revision = Some(revision);
        }
    }

    /// Requests the panels made this frame that need the shell's state.
    fn handle_requests(&mut self) {
        if self.gui_state.project_generation != self.seen_project_generation {
            self.seen_project_generation = self.gui_state.project_generation;
            self.on_project_opened();
        }

        // Another panel (hierarchy, inspector, files) asked to open a script
        if let Some(path) = self.gui_state.open_script_request.take() {
            self.open_script(path);
        }

        if let Some(path) = self.gui_state.deleted_file.take()
            && self.current_edited_file.as_ref() == Some(&path)
        {
            self.close_script();
        }

        if std::mem::take(&mut self.gui_state.save_requested) {
            let script_saved = self.save_editor_if_dirty();
            if script_saved && self.gui_state.save_project().is_ok() {
                LOGGER.info("Project saved");
            }
        }

        if std::mem::take(&mut self.gui_state.build_requested) {
            // Build what's on screen: unsaved script edits included
            if self.save_editor_if_dirty() && self.gui_state.save_project().is_ok() {
                crate::menus::project::start_build(&mut self.gui_state);
            }
        }
    }

    /// Reset everything that belonged to the previous project.
    fn on_project_opened(&mut self) {
        // The buffer belongs to the previous project; its path is absolute,
        // so saving it now still writes to the right file
        self.save_editor_if_dirty();
        self.close_script();
        self.show_editor = false;
        self.game_runtime.reset();
        self.render_engine.cleanup();
        self.viewport_drag = None;
        self.inspector.reset();
        self.scene_hierarchy.popup_manager.close_all();
        self.file_system.reset();
    }

    fn show_windows(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.content_rect();
        let spacing = 4.0;
        let min_side_panel_width = 200.0;

        // Scene undo/redo shortcuts, only when no text field is focused
        // (focused text fields have their own undo). The more specific
        // shortcut is checked first: `consume_shortcut` ignores extra Shift.
        if ctx.memory(|m| m.focused().is_none()) {
            let redo_pressed = ctx.input_mut(|i| {
                i.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::Z,
                )) || i.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND,
                    egui::Key::Y,
                ))
            });
            let undo_pressed = ctx.input_mut(|i| {
                i.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND,
                    egui::Key::Z,
                ))
            });
            if redo_pressed {
                self.gui_state.redo();
            } else if undo_pressed {
                self.gui_state.undo();
            }
        }

        // Another panel asked to insert a snippet at the editor cursor
        if let Some(snippet) = self.gui_state.script_insert_request.take() {
            self.insert_into_editor(ctx, &snippet);
        }

        self.set_theme(ctx);

        // Viewport (Center)
        egui::Window::new("Main Window")
            .frame(egui::Frame::NONE)
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
                        .frame(egui::Frame::NONE)
                        .show(ui, |ui| {
                            // Use vertical layout to split the panel
                            egui::Panel::top("scene_panel")
                                .resizable(true)
                                .min_size(200.0)
                                .max_size(ui.available_height() * 0.75)
                                .default_size(ui.available_height() * 0.5)
                                .frame(egui::Frame::NONE)
                                .show(ui, |ui| {
                                    self.scene_hierarchy.show(ctx, ui, &mut self.gui_state);
                                });

                            // Add the file system view in the bottom part
                            egui::CentralPanel::default()
                                .frame(egui::Frame::NONE)
                                .show(ui, |ui| {
                                    self.file_system.show(ctx, ui, &mut self.gui_state);
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
                        .frame(egui::Frame::NONE.outer_margin(inspector_margin))
                        .show(ui, |ui| {
                            ui.heading("Inspector");
                            ui.separator();
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                self.inspector.show(ctx, ui, &mut self.gui_state);
                            });
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
    /// Runs before every frame and also while the window is minimized or
    /// hidden (when `ui` doesn't run), so closing from the taskbar still
    /// gets the unsaved-changes prompt.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.intercept_close(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frame(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::SelectedItem;
    use rust_2d_game_engine::scene::{AttributeType, AttributeValue};

    /// Run `frames` editor frames headlessly at the given window width.
    fn run_frames(app: &mut EditorApp, ctx: &egui::Context, width: f32, frames: usize) {
        for _ in 0..frames {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 720.0),
                )),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| app.frame(ui));
            // Nothing renders here; textures don't need uploading
            output.textures_delta.clear();
        }
    }

    fn app_with_project() -> (egui::Context, EditorApp, PathBuf) {
        let ctx = egui::Context::default();
        let mut app = EditorApp::new(&ctx);
        run_frames(&mut app, &ctx, 1280.0, 2);

        let project = std::env::temp_dir()
            .join(format!("rust2d_editor_{}", uuid::Uuid::new_v4()))
            .join("smoke");
        let loaded = crate::project::create_project(&project).expect("create project");
        app.gui_state.open_project(loaded);
        run_frames(&mut app, &ctx, 1280.0, 2);
        (ctx, app, project)
    }

    #[test]
    fn test_editor_panels_survive_awkward_content() {
        let (ctx, mut app, project) = app_with_project();

        // A selected entity with a long non-ASCII attribute name, shown in a
        // narrow inspector (truncation used to slice inside a character)
        let scene_id = app
            .gui_state
            .scene_manager
            .as_ref()
            .unwrap()
            .active_scene
            .unwrap();
        let entity_id = {
            let scene = app
                .gui_state
                .scene_manager
                .as_mut()
                .unwrap()
                .get_scene_mut(scene_id)
                .unwrap();
            let id = scene.create_entity("角色").unwrap();
            scene
                .get_entity_mut(id)
                .unwrap()
                .create_attribute(
                    &"最大速度".repeat(12),
                    AttributeType::Float,
                    AttributeValue::Float(1.0),
                )
                .unwrap();
            id
        };
        app.gui_state.select_entity(scene_id, entity_id);
        run_frames(&mut app, &ctx, 1280.0, 3);
        run_frames(&mut app, &ctx, 820.0, 3);

        // An image wider than the GPU texture limit, selected in the files panel
        let big = project.join("assets").join("images").join("big.png");
        image::RgbaImage::new(9000, 4).save(&big).unwrap();
        app.gui_state.selected_item = SelectedItem::File(big);
        run_frames(&mut app, &ctx, 1280.0, 2);

        // Play mode
        app.sync_scene_manager_to_runtime();
        app.game_runtime.run().expect("the starter scene plays");
        run_frames(&mut app, &ctx, 1280.0, 3);
        app.game_runtime.reset();
        run_frames(&mut app, &ctx, 1280.0, 1);
    }

    #[test]
    fn test_reopening_the_open_script_keeps_unsaved_edits() {
        let (ctx, mut app, project) = app_with_project();
        let scripts = project.join("assets").join("scripts");
        let first = scripts.join("first.lua");
        let second = scripts.join("second.lua");
        std::fs::write(&first, "-- first").unwrap();
        std::fs::write(&second, "-- second").unwrap();

        app.open_script(first.clone());
        app.editor_content = "-- edited".to_string();
        app.editor_dirty = true;

        // Opening the same file again just shows it
        app.open_script(first.clone());
        assert_eq!(app.editor_content, "-- edited");
        assert!(app.editor_dirty);

        // Switching files saves the edits first
        app.open_script(second.clone());
        assert_eq!(std::fs::read_to_string(&first).unwrap(), "-- edited");
        assert_eq!(app.editor_content, "-- second");
        run_frames(&mut app, &ctx, 1280.0, 2);
    }

    #[test]
    fn test_opening_another_project_resets_the_session() {
        let (ctx, mut app, project) = app_with_project();
        let script = project.join("assets").join("scripts").join("a.lua");
        std::fs::write(&script, "-- a").unwrap();
        app.open_script(script.clone());
        app.editor_content = "-- unsaved".to_string();
        app.editor_dirty = true;
        app.sync_scene_manager_to_runtime();
        app.game_runtime.run().unwrap();

        let other = project.with_file_name("other");
        let loaded = crate::project::create_project(&other).unwrap();
        app.gui_state.open_project(loaded);
        run_frames(&mut app, &ctx, 1280.0, 2);

        assert_eq!(
            app.game_runtime.get_state(),
            rust_2d_game_engine::runtime::RuntimeState::Stopped
        );
        assert!(app.current_edited_file.is_none());
        assert_eq!(std::fs::read_to_string(&script).unwrap(), "-- unsaved");
    }

    /// The game camera's frame as the viewport paints it in one frame: its
    /// red 2 px line segments.
    fn painted_camera_frame(app: &mut EditorApp, ctx: &egui::Context) -> Vec<[egui::Pos2; 2]> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 720.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.frame(ui));
        output.textures_delta.clear();
        // Pure red, whatever its opacity: the main window is still fading in
        // after the few frames a test runs
        let red = |color: egui::Color32| color.r() > 0 && color.g() == 0 && color.b() == 0;
        output
            .shapes
            .into_iter()
            .filter_map(|clipped| match clipped.shape {
                egui::Shape::LineSegment { points, stroke }
                    if red(stroke.color) && stroke.width == 2.0 =>
                {
                    Some(points)
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_play_mode_frames_the_camera_where_the_game_moved_it() {
        let (ctx, mut app, project) = app_with_project();
        let script = project.join("assets").join("scripts").join("pan.lua");
        std::fs::write(
            &script,
            r#"
            function update(scene_id, entity_id)
                if script_state.state.pan then
                    script_state.state.pan = false
                    set_x(scene_id, entity_id, get_attribute(scene_id, entity_id, "x") + 100)
                end
            end
            "#,
        )
        .unwrap();
        {
            let manager = app.gui_state.scene_manager.as_mut().unwrap();
            let scene = manager.get_active_scene_mut().unwrap();
            let camera = scene.default_camera.unwrap();
            scene
                .get_entity_mut(camera)
                .unwrap()
                .set_script(script)
                .unwrap();
        }
        app.sync_scene_manager_to_runtime();
        app.game_runtime.run().unwrap();

        let before = painted_camera_frame(&mut app, &ctx);
        assert_eq!(before.len(), 4, "the frame's four sides");
        app.game_runtime
            .scripting()
            .lua()
            .load("script_state.state.pan = true")
            .exec()
            .unwrap();
        let after = painted_camera_frame(&mut app, &ctx);

        // The game panned its camera 100 world units to the right
        let shift = 100.0 * app.render_engine.camera.zoom;
        assert_eq!(after.len(), 4);
        for (old, new) in before.iter().zip(&after) {
            for (a, b) in old.iter().zip(new) {
                assert!(
                    (b.x - a.x - shift).abs() < 0.01 && (b.y - a.y).abs() < 0.01,
                    "{:?} -> {:?}",
                    a,
                    b
                );
            }
        }
        app.game_runtime.reset();
    }
}
