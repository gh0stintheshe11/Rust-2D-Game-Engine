//! The Lua script editor panel: syntax-highlighted editing with line
//! numbers, live syntax checking, an engine API palette, and snippet
//! insertion driven by other panels (e.g. clicking an attribute in the
//! inspector inserts a `get_attribute` call at the cursor).

use super::EngineGui;
use crate::logger::LOGGER;
use eframe::egui;

/// One palette entry: (label, snippet, doc).
type ApiEntry = (&'static str, &'static str, &'static str);

/// Engine API reference shown in the palette, grouped by subsystem.
const API_GROUPS: &[(&str, &[ApiEntry])] = &[
    (
        "Input",
        &[
            (
                "is_key_just_pressed",
                "is_key_just_pressed(\"Space\")",
                "True only on the frame the key goes down",
            ),
            (
                "is_key_pressed",
                "is_key_pressed(\"A\")",
                "True while the key is held",
            ),
            (
                "is_mouse_pressed",
                "is_mouse_pressed(\"left\")",
                "\"left\", \"right\" or \"middle\"",
            ),
            (
                "get_mouse_position",
                "get_mouse_position()",
                "Returns {x, y} in window coordinates",
            ),
            (
                "get_scroll_delta",
                "get_scroll_delta()",
                "Returns {x, y}; zero when not scrolling",
            ),
        ],
    ),
    (
        "Physics",
        &[
            (
                "set_velocity",
                "set_velocity(entity_id, 0.0, 0.0)",
                "Set the entity's velocity (+y is down)",
            ),
            (
                "apply_force",
                "apply_force(entity_id, 0.0, 0.0)",
                "Continuous force; reset after every physics step",
            ),
            (
                "apply_impulse",
                "apply_impulse(entity_id, 0.0, 0.0)",
                "Instant velocity change",
            ),
            (
                "get_colliding_entities",
                "get_colliding_entities(entity_id)",
                "Array of entity ids currently in contact",
            ),
            (
                "set_gravity",
                "set_gravity(0.0, 50.0)",
                "Change the global gravity vector",
            ),
            (
                "add_entity_to_physics_engine",
                "add_entity_to_physics_engine(entity_id)",
                "(Re)build the entity's physics body from its attributes",
            ),
            (
                "remove_entity_from_physics_engine",
                "remove_entity_from_physics_engine(entity_id)",
                "Remove the entity's physics body",
            ),
        ],
    ),
    (
        "Entities",
        &[
            (
                "add_entity",
                "add_entity(scene_id, \"name\")",
                "Create an entity; returns its id",
            ),
            (
                "remove_entity",
                "remove_entity(scene_id, entity_id)",
                "Delete an entity from the scene",
            ),
            (
                "create_physical_entity",
                "create_physical_entity(scene_id, \"name\", 0.0, 0.0, 0.0)",
                "Entity with physics attributes at x, y, z",
            ),
            (
                "get_entity_name",
                "get_entity_name(scene_id, entity_id)",
                "Name string, or nil if the entity is gone",
            ),
            (
                "set_position",
                "set_position(scene_id, entity_id, 0.0, 0.0)",
                "Set x and y (z untouched)",
            ),
            (
                "add_image",
                "add_image(entity_id, \"assets/images/sprite.png\")",
                "Attach a sprite (path relative to the project)",
            ),
            (
                "set_script",
                "set_script(entity_id, \"assets/scripts/logic.lua\")",
                "Attach a script (path relative to the project)",
            ),
            (
                "list_entities_name_x_y",
                "list_entities_name_x_y(scene_id)",
                "Array of {id, name, x, y} for every entity",
            ),
        ],
    ),
    (
        "Attributes",
        &[
            (
                "get_attribute",
                "get_attribute(scene_id, entity_id, \"name\")",
                "Read any attribute: number/bool/string/{x, y}, nil if missing",
            ),
            (
                "set_attribute",
                "set_attribute(scene_id, entity_id, \"name\", value)",
                "Write an attribute; the value must match its declared type",
            ),
            (
                "has_attribute",
                "has_attribute(scene_id, entity_id, \"name\")",
                "True if the entity has the attribute",
            ),
            (
                "create_attribute_float",
                "create_attribute_float(scene_id, entity_id, \"name\", 0.0)",
                "Add a Float attribute",
            ),
            (
                "create_attribute_bool",
                "create_attribute_bool(scene_id, entity_id, \"name\", false)",
                "Add a Boolean attribute",
            ),
            (
                "create_attribute_vector2",
                "create_attribute_vector2(scene_id, entity_id, \"name\", 0.0, 0.0)",
                "Add a Vector2 attribute",
            ),
        ],
    ),
    (
        "Audio",
        &[
            (
                "play_sound",
                "play_sound(\"assets/sounds/effect.ogg\")",
                "Returns a play id, or nil if playback isn't possible",
            ),
            (
                "stop_sound",
                "stop_sound(play_id)",
                "Stop one playing sound",
            ),
            (
                "is_sound_playing",
                "is_sound_playing(play_id)",
                "True while the sound plays",
            ),
            ("stop_all_sounds", "stop_all_sounds()", "Stop everything"),
        ],
    ),
    (
        "Game flow",
        &[
            (
                "end_game",
                "end_game()",
                "Game over: freezes on this frame; only Reset exits",
            ),
            (
                "accumulated_time",
                "accumulated_time",
                "Global: real seconds since play started",
            ),
            (
                "script_state",
                "script_state.state.my_key",
                "Global table shared by all scripts, persists for the session",
            ),
        ],
    ),
];

const SCRIPT_TEMPLATE: &str = r#"-- Runs once, before this entity's first update
function init(scene_id, entity_id)

end

-- Runs every frame
function update(scene_id, entity_id)

end

-- Runs when this entity starts touching another physics entity
function on_collision(scene_id, entity_id, other_id)

end
"#;

/// Parse the buffer with Lua and return a human-readable error, if any.
pub fn check_script_syntax(source: &str) -> Option<String> {
    let lua = mlua::Lua::new();
    match lua.load(source).set_name("script").into_function() {
        Ok(_) => None,
        Err(e) => {
            let message = e.to_string();
            // mlua reports as `[string "script"]:LINE: message`; make it friendlier
            let message = message.replace("[string \"script\"]:", "line ");
            // Keep only the first line of the (possibly multi-line) report
            Some(message.lines().next().unwrap_or(&message).to_string())
        }
    }
}

impl EngineGui {
    /// Insert `text` at the current cursor position of the script editor
    /// (or at the end of the buffer when there's no cursor state yet).
    pub(super) fn insert_into_editor(&mut self, ctx: &egui::Context, text: &str) {
        if self.current_edited_file.is_none() {
            LOGGER.info("Open a script in the editor first, then insert snippets.");
            return;
        }

        let id = egui::Id::new("script_editor_text");
        let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();

        let char_index = state
            .cursor
            .char_range()
            .map(|range| range.primary.index.0)
            .unwrap_or_else(|| self.editor_content.chars().count())
            .min(self.editor_content.chars().count());

        let byte_index = self
            .editor_content
            .char_indices()
            .nth(char_index)
            .map(|(byte, _)| byte)
            .unwrap_or(self.editor_content.len());

        self.editor_content.insert_str(byte_index, text);
        self.editor_dirty = true;
        self.syntax_error = check_script_syntax(&self.editor_content);

        // Place the cursor after the inserted snippet
        let new_cursor =
            egui::text::CCursor::new(egui::text::CharIndex(char_index + text.chars().count()));
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(new_cursor)));
        state.store(ctx, id);

        // Make sure the editor is visible
        self.show_editor = true;
    }

    /// The script editor panel (center area, Editor tab).
    pub(super) fn show_script_editor(&mut self, ui: &mut egui::Ui, content_rect: egui::Rect) {
        ui.painter()
            .rect_filled(content_rect, 0.0, egui::Color32::from_gray(40));

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

        // Header: filename, dirty indicator, Ln/Col, API palette toggle
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

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.toggle_value(&mut self.show_api_palette, "📖 API")
                    .on_hover_text("Engine functions reference; click to insert at the cursor");

                // Ln/Col of the primary cursor
                if let Some(state) = egui::text_edit::TextEditState::load(
                    ui.ctx(),
                    egui::Id::new("script_editor_text"),
                ) {
                    if let Some(range) = state.cursor.char_range() {
                        let index = range.primary.index.0;
                        let mut line = 1usize;
                        let mut column = 1usize;
                        for c in self.editor_content.chars().take(index) {
                            if c == '\n' {
                                line += 1;
                                column = 1;
                            } else {
                                column += 1;
                            }
                        }
                        ui.weak(format!("Ln {}, Col {}", line, column));
                    }
                }
            });
        });

        // Empty file: offer the lifecycle template
        if self.current_edited_file.is_some() && self.editor_content.trim().is_empty() {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.weak("Empty script —");
                if ui.link("insert the lifecycle template").clicked() {
                    self.editor_content = SCRIPT_TEMPLATE.to_string();
                    self.editor_dirty = true;
                    self.syntax_error = check_script_syntax(&self.editor_content);
                }
            });
        }

        // Optional API palette on the right
        if self.show_api_palette {
            egui::Panel::right("script_api_palette")
                .resizable(true)
                .default_size(230.0)
                .min_size(170.0)
                .max_size(content_rect.width() * 0.5)
                .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(6, 4)))
                .show(ui, |ui| {
                    self.show_api_palette_contents(ui);
                });
        }

        let theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
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

        // Syntax status bar (shown under the header, above the code)
        if let Some(error) = &self.syntax_error {
            ui.colored_label(
                egui::Color32::from_rgb(255, 120, 120),
                format!("⛔ {}", error),
            );
        }

        let line_count = self.editor_content.lines().count().max(1);
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        let gutter_width = (line_count.to_string().len().max(2) as f32) * 8.0 + 8.0;

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    // Line number gutter (same monospace metrics as the code)
                    let numbers: String = (1..=line_count)
                        .map(|n| n.to_string())
                        .collect::<Vec<_>>()
                        .join("\n");
                    ui.allocate_ui_with_layout(
                        egui::vec2(gutter_width, row_height * line_count as f32),
                        egui::Layout::top_down(egui::Align::Max),
                        |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(numbers)
                                        .monospace()
                                        .color(egui::Color32::from_gray(110)),
                                )
                                .wrap_mode(egui::TextWrapMode::Extend),
                            );
                        },
                    );

                    let response = ui.add_sized(
                        egui::vec2(
                            (ui.available_width()).max(200.0),
                            (row_height * line_count as f32).max(ui.available_height()),
                        ),
                        egui::TextEdit::multiline(&mut self.editor_content)
                            .id(egui::Id::new("script_editor_text"))
                            .code_editor()
                            .lock_focus(true)
                            .desired_width(f32::INFINITY)
                            .layouter(&mut layouter),
                    );

                    // Mark dirty on edits; the buffer is written on Ctrl+S,
                    // focus loss, or when switching files/views
                    if response.changed() {
                        self.editor_dirty = true;
                        self.syntax_error = check_script_syntax(&self.editor_content);
                    }
                    if response.lost_focus() {
                        self.save_editor_if_dirty();
                    }
                });
            });
    }

    fn show_api_palette_contents(&mut self, ui: &mut egui::Ui) {
        ui.heading("Engine API");
        ui.weak("Click to insert at the cursor");
        ui.separator();

        let mut pending_insert: Option<&str> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (group, functions) in API_GROUPS {
                    egui::CollapsingHeader::new(*group)
                        .default_open(true)
                        .show(ui, |ui| {
                            for (label, snippet, doc) in *functions {
                                let response = ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new(*label).monospace().size(12.0),
                                        )
                                        .frame(false),
                                    )
                                    .on_hover_text(format!("{}\n\n{}", snippet, doc));
                                if response.clicked() {
                                    pending_insert = Some(snippet);
                                }
                            }
                        });
                }
            });

        if let Some(snippet) = pending_insert {
            let ctx = ui.ctx().clone();
            self.insert_into_editor(&ctx, snippet);
        }
    }
}
