//! The editor application: window layout and the state tying the panels,
//! the script editor and the game runtime together.

use crate::menu_bar::MenuBar;
use crate::panels::files::FileSystem;
use crate::panels::hierarchy::SceneHierarchy;
use crate::panels::hierarchy::popup::PendingDelete;
use crate::panels::inspector::Inspector;
use crate::state::{GuiState, SelectedItem};
use eframe::egui;
use rust_2d_game_engine::logger::{ConsoleMessage, ConsoleMessageType, LOGGER};
use rust_2d_game_engine::{
    audio::AudioEngine,
    input::InputHandler,
    physics::PhysicsEngine,
    render::RenderEngine,
    runtime::{GameRuntime, RuntimeState},
    scene::SceneManager,
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

    /// Duplicate (Ctrl+D), copy and paste (the platform's clipboard
    /// shortcuts) and delete (Delete or Backspace, after confirmation) the
    /// selected entity. Pastes go into the selected scene, or the active one.
    fn handle_entity_shortcuts(&mut self, ctx: &egui::Context) {
        let (duplicate, delete, copy, paste) = ctx.input_mut(|i| {
            let duplicate = i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND,
                egui::Key::D,
            ));
            let delete = i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)
                || i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace);
            let copy = i.events.iter().any(|e| matches!(e, egui::Event::Copy));
            let paste = i.events.iter().find_map(|e| match e {
                egui::Event::Paste(text) => Some(text.clone()),
                _ => None,
            });
            (duplicate, delete, copy, paste)
        });

        if let SelectedItem::Entity(scene_id, entity_id) = self.gui_state.selected_item {
            if copy && let Some(text) = self.gui_state.copy_entity(scene_id, entity_id) {
                ctx.copy_text(text);
            }
            if duplicate {
                self.gui_state.duplicate_entity(scene_id, entity_id);
            } else if delete {
                let name = self
                    .gui_state
                    .scene_manager
                    .as_ref()
                    .and_then(|manager| manager.get_scene(scene_id))
                    .and_then(|scene| scene.get_entity(entity_id).ok())
                    .map(|entity| entity.name.clone());
                if let Some(name) = name {
                    self.scene_hierarchy.popup_manager.pending_delete =
                        Some(PendingDelete::Entity(scene_id, entity_id, name));
                }
            }
        }

        if let Some(text) = paste
            && let Some(entity) =
                crate::clipboard::from_clipboard_text(&text, &self.gui_state.project_path)
            && let Some(scene_id) = self.gui_state.selected_scene().or_else(|| {
                self.gui_state
                    .scene_manager
                    .as_ref()
                    .and_then(|manager| manager.active_scene)
            })
        {
            self.gui_state.paste_entity(scene_id, entity);
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

            // While the game plays, keys belong to it
            if self.game_runtime.get_state() != RuntimeState::Playing {
                self.handle_entity_shortcuts(ctx);
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

    /// One frame with `events`, returning egui's output.
    fn frame_with(
        app: &mut EditorApp,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 720.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.frame(ui));
        output.textures_delta.clear();
        output
    }

    fn key_press(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    /// (x, y) of every entity named `name` in a scene, sorted.
    fn positions(app: &EditorApp, scene_id: uuid::Uuid, name: &str) -> Vec<(f32, f32)> {
        let manager = app.gui_state.scene_manager.as_ref().unwrap();
        let mut found: Vec<(f32, f32)> = manager
            .get_scene(scene_id)
            .unwrap()
            .entities
            .values()
            .filter(|entity| entity.name == name)
            .map(|entity| (entity.get_x(), entity.get_y()))
            .collect();
        found.sort_by(|a, b| a.partial_cmp(b).unwrap());
        found
    }

    #[test]
    fn test_duplicate_copy_paste_and_delete_shortcuts() {
        use rust_2d_game_engine::scene::AttributeValue;

        let (ctx, mut app, _project) = app_with_project();
        let (scene_id, coin, level) = {
            let manager = app.gui_state.scene_manager.as_mut().unwrap();
            let level = manager.create_scene("level 2").unwrap();
            let scene_id = manager.active_scene.unwrap();
            let scene = manager.get_scene_mut(scene_id).unwrap();
            let coin = scene.create_entity("coin").unwrap();
            let entity = scene.get_entity_mut(coin).unwrap();
            entity.set_position(40.0, 60.0, 0.0).unwrap();
            entity
                .set_or_create_attribute("value", AttributeValue::Integer(5))
                .unwrap();
            (scene_id, coin, level)
        };
        app.gui_state.commit_change();
        app.gui_state.select_entity(scene_id, coin);

        // Ctrl+D: a copy, moved off the original, selected; one undo step
        frame_with(
            &mut app,
            &ctx,
            vec![key_press(egui::Key::D, egui::Modifiers::COMMAND)],
        );
        assert_eq!(
            positions(&app, scene_id, "coin"),
            vec![(40.0, 60.0), (56.0, 76.0)]
        );
        let crate::state::SelectedItem::Entity(_, copy) = app.gui_state.selected_item else {
            panic!("the copy is selected");
        };
        assert_ne!(copy, coin);
        app.gui_state.undo();
        assert_eq!(positions(&app, scene_id, "coin"), vec![(40.0, 60.0)]);

        // Copy puts the entity on the system clipboard
        app.gui_state.select_entity(scene_id, coin);
        let output = frame_with(&mut app, &ctx, vec![egui::Event::Copy]);
        let text = output
            .platform_output
            .commands
            .iter()
            .find_map(|command| match command {
                egui::OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            })
            .expect("copied to the clipboard");

        // Pasted into another scene: same place; into its own scene: moved
        app.gui_state.select_scene(level);
        frame_with(&mut app, &ctx, vec![egui::Event::Paste(text.clone())]);
        assert_eq!(positions(&app, level, "coin"), vec![(40.0, 60.0)]);
        let pasted = {
            let manager = app.gui_state.scene_manager.as_ref().unwrap();
            let scene = manager.get_scene(level).unwrap();
            let coin = scene.entities.values().find(|e| e.name == "coin").unwrap();
            coin.get_float("value")
        };
        assert_eq!(pasted, Some(5.0), "attributes come along");
        app.gui_state.select_scene(scene_id);
        frame_with(&mut app, &ctx, vec![egui::Event::Paste(text)]);
        assert_eq!(
            positions(&app, scene_id, "coin"),
            vec![(40.0, 60.0), (56.0, 76.0)]
        );

        // Other text on the clipboard isn't an entity
        frame_with(
            &mut app,
            &ctx,
            vec![egui::Event::Paste("hello".to_string())],
        );
        assert_eq!(positions(&app, scene_id, "coin").len(), 2);

        // The menu's Paste uses the last entity copied
        assert!(app.gui_state.paste_copied_entity(level).is_some());
        assert_eq!(
            positions(&app, level, "coin"),
            vec![(40.0, 60.0), (56.0, 76.0)]
        );

        // Delete asks first
        app.gui_state.select_entity(scene_id, coin);
        frame_with(
            &mut app,
            &ctx,
            vec![key_press(egui::Key::Delete, egui::Modifiers::NONE)],
        );
        assert!(matches!(
            app.scene_hierarchy.popup_manager.pending_delete,
            Some(PendingDelete::Entity(_, id, _)) if id == coin
        ));
        app.scene_hierarchy.popup_manager.pending_delete = None;

        // While the game plays, the keys are the game's
        app.sync_scene_manager_to_runtime();
        app.game_runtime.run().unwrap();
        app.gui_state.select_entity(scene_id, coin);
        frame_with(
            &mut app,
            &ctx,
            vec![
                key_press(egui::Key::Delete, egui::Modifiers::NONE),
                key_press(egui::Key::D, egui::Modifiers::COMMAND),
            ],
        );
        assert!(app.scene_hierarchy.popup_manager.pending_delete.is_none());
        assert_eq!(positions(&app, scene_id, "coin").len(), 2);
        app.game_runtime.reset();
    }

    /// One frame with `events` and `modifiers` held.
    fn frame_with_modifiers(
        app: &mut EditorApp,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        modifiers: egui::Modifiers,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 720.0),
            )),
            events: std::iter::once(egui::Event::ModifiersChanged(modifiers))
                .chain(events)
                .collect(),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.frame(ui));
        output.textures_delta.clear();
        output
    }

    /// Drag with the left button from `from` by `delta` screen pixels.
    fn drag(
        app: &mut EditorApp,
        ctx: &egui::Context,
        from: egui::Pos2,
        delta: egui::Vec2,
        modifiers: egui::Modifiers,
    ) {
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        };
        let mut frames = vec![
            vec![egui::Event::PointerMoved(from)],
            vec![button(from, true)],
        ];
        for step in 1..=5 {
            frames.push(vec![egui::Event::PointerMoved(
                from + delta * step as f32 / 5.0,
            )]);
        }
        frames.push(vec![button(from + delta, false)]);
        frames.push(vec![]);
        for events in frames {
            frame_with_modifiers(app, ctx, events, modifiers);
        }
    }

    #[test]
    fn test_dragging_snaps_to_the_grid_unless_shift_is_held() {
        let (ctx, mut app, project) = app_with_project();
        let image = project.join("assets").join("images").join("box.png");
        image::RgbaImage::new(40, 40).save(&image).unwrap();
        let (scene_id, id) = {
            let manager = app.gui_state.scene_manager.as_mut().unwrap();
            let scene_id = manager.active_scene.unwrap();
            let scene = manager.get_scene_mut(scene_id).unwrap();
            let id = scene.create_entity("box").unwrap();
            let entity = scene.get_entity_mut(id).unwrap();
            entity.set_position(200.0, 150.0, 0.0).unwrap();
            entity.add_image(image).unwrap();
            (scene_id, id)
        };
        app.gui_state.commit_change();

        // The sprite on screen: the only mesh with an image texture (text,
        // e.g. the console's, is drawn from the font texture)
        let output = frame_with(&mut app, &ctx, vec![]);
        let sprite = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Mesh(mesh) if mesh.texture_id != egui::TextureId::default() => {
                    Some(mesh.calc_bounds())
                }
                _ => None,
            })
            .expect("the sprite is painted");
        let zoom = app.render_engine.camera.zoom;
        let position = |app: &EditorApp| {
            let manager = app.gui_state.scene_manager.as_ref().unwrap();
            let entity = manager.get_scene(scene_id).unwrap().get_entity(id).unwrap();
            (entity.get_x(), entity.get_y())
        };

        // Snapping is on by default, to the 32 px grid
        drag(
            &mut app,
            &ctx,
            sprite.center(),
            egui::vec2(60.0, 45.0),
            egui::Modifiers::NONE,
        );
        let (x, y) = position(&app);
        let raw = (200.0 + 60.0 / zoom, 150.0 + 45.0 / zoom);
        assert_eq!(
            (x % 32.0, y % 32.0),
            (0.0, 0.0),
            "on the grid: ({}, {})",
            x,
            y
        );
        assert!((x - raw.0).abs() <= 16.0 && (y - raw.1).abs() <= 16.0);

        // Shift drags freely
        let from = sprite.center() + egui::vec2(60.0, 45.0);
        drag(
            &mut app,
            &ctx,
            from,
            egui::vec2(13.0, 7.0),
            egui::Modifiers::SHIFT,
        );
        let (free_x, free_y) = position(&app);
        assert!(
            (free_x - (x + 13.0 / zoom)).abs() < 0.01 && (free_y - (y + 7.0 / zoom)).abs() < 0.01,
            "({}, {}) from ({}, {})",
            free_x,
            free_y,
            x,
            y
        );
    }

    /// Where a label with exactly this text was painted.
    fn label_rect(output: &egui::FullOutput, text: &str) -> Option<egui::Rect> {
        output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) if shape.galley.text() == text => {
                    Some(shape.visual_bounding_rect())
                }
                _ => None,
            })
    }

    /// Click the label with this text (as painted in the last frame), with
    /// `button`, and run frames until whatever it opened has been drawn;
    /// returns the last frame's output.
    fn click_label(
        app: &mut EditorApp,
        ctx: &egui::Context,
        output: &egui::FullOutput,
        text: &str,
        button: egui::PointerButton,
    ) -> egui::FullOutput {
        let pos = label_rect(output, text)
            .unwrap_or_else(|| panic!("no label {:?} on screen", text))
            .center();
        let press = |pressed| egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame_with(app, ctx, vec![egui::Event::PointerMoved(pos)]);
        frame_with(app, ctx, vec![press(true)]);
        frame_with(app, ctx, vec![press(false)]);
        frame_with(app, ctx, vec![]);
        frame_with(app, ctx, vec![])
    }

    /// Hover the label with this text until its submenu is drawn.
    fn hover_label(
        app: &mut EditorApp,
        ctx: &egui::Context,
        output: &egui::FullOutput,
        text: &str,
    ) -> egui::FullOutput {
        let pos = label_rect(output, text)
            .unwrap_or_else(|| panic!("no label {:?} on screen", text))
            .center();
        for _ in 0..4 {
            frame_with(app, ctx, vec![egui::Event::PointerMoved(pos)]);
        }
        frame_with(app, ctx, vec![])
    }

    #[test]
    fn test_presets_context_and_grid_menus() {
        let (ctx, mut app, _project) = app_with_project();
        let left = egui::PointerButton::Primary;
        let (scene_id, zone) = {
            let manager = app.gui_state.scene_manager.as_mut().unwrap();
            let scene_id = manager.active_scene.unwrap();
            let zone = manager
                .get_scene_mut(scene_id)
                .unwrap()
                .create_entity("zone")
                .unwrap();
            (scene_id, zone)
        };
        app.gui_state.commit_change();
        app.gui_state.select_entity(scene_id, zone);
        let entity = |app: &EditorApp, id| {
            let manager = app.gui_state.scene_manager.as_ref().unwrap();
            manager
                .get_scene(scene_id)
                .unwrap()
                .get_entity(id)
                .unwrap()
                .clone()
        };

        // Inspector: Add Feature > Physics > Trigger zone
        let output = frame_with(&mut app, &ctx, vec![]);
        let output = click_label(&mut app, &ctx, &output, "Add Feature", left);
        let output = hover_label(&mut app, &ctx, &output, "Physics");
        click_label(&mut app, &ctx, &output, "Trigger zone", left);
        let zone_entity = entity(&app, zone);
        assert_eq!(zone_entity.get_bool("is_sensor"), Some(true));
        assert_eq!(zone_entity.get_bool("has_collision"), Some(true));
        assert!(app.gui_state.undo_stack.can_undo(), "an undo step");

        // Hierarchy: right-click > Duplicate
        let output = frame_with(&mut app, &ctx, vec![]);
        let output = click_label(
            &mut app,
            &ctx,
            &output,
            "🖼 zone",
            egui::PointerButton::Secondary,
        );
        click_label(&mut app, &ctx, &output, "Duplicate", left);
        assert_eq!(positions(&app, scene_id, "zone").len(), 2);

        // View > Grid > 16 px
        let output = frame_with(&mut app, &ctx, vec![]);
        let output = click_label(&mut app, &ctx, &output, "View", left);
        let output = hover_label(&mut app, &ctx, &output, "Grid");
        click_label(&mut app, &ctx, &output, "16 px", left);
        assert_eq!(app.gui_state.grid_size, 16.0);
    }
}
