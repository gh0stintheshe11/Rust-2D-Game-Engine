//! The scene viewport: editing view, play mode, and play controls.

use super::EditorApp;
use eframe::egui;
use rust_2d_game_engine::logger::LOGGER;
use rust_2d_game_engine::runtime::{CameraMode, RuntimeState};

impl EditorApp {
    /// The center area: the script editor, or the scene viewport with the
    /// play controls.
    pub(super) fn show_center_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let content_rect = ui.available_rect_before_wrap();

        // First fill the background
        if self.show_editor {
            self.show_script_editor(ui, content_rect);
        } else {
            // Render only the viewport content when play in the GUI.
            // Paused/Ended still route to the runtime so the freeze
            // frame of the running game stays visible.
            let runtime_state = self.game_runtime.get_state();
            if matches!(
                runtime_state,
                RuntimeState::Playing | RuntimeState::Paused | RuntimeState::Ended
            ) {
                // The game is shown through the editor's camera
                let camera = self.render_engine.camera;
                self.game_runtime.set_camera_mode(CameraMode::Free {
                    position: camera.position,
                    zoom: camera.zoom,
                });
                self.game_runtime
                    .set_show_colliders(self.gui_state.show_colliders);

                let game_view_rect = ui.available_rect_before_wrap();
                self.game_runtime.update(ctx, ui, game_view_rect);

                // Then draw the game camera bounds, from the game's own
                // scenes: scripts may move the camera or switch scenes
                let camera_lines = self
                    .game_runtime
                    .get_scene_manager()
                    .get_active_scene()
                    .map(|scene| self.render_engine.get_game_camera_bounds(scene));
                if let Some(camera_lines) = camera_lines {
                    for (start, end) in camera_lines {
                        ui.painter().line_segment(
                            [
                                egui::pos2(
                                    content_rect.min.x + start.0,
                                    content_rect.min.y + start.1,
                                ),
                                egui::pos2(content_rect.min.x + end.0, content_rect.min.y + end.1),
                            ],
                            egui::Stroke::new(2.0_f32, egui::Color32::RED),
                        );
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
                        if !self.gui_state.project_open {
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
                                    if !self.save_editor_if_dirty() {
                                        LOGGER
                                            .warning("Playing without the unsaved script changes");
                                    }
                                    // Sync scene manager before starting
                                    self.sync_scene_manager_to_runtime();

                                    match self.game_runtime.run() {
                                        Ok(_) => {
                                            LOGGER.info("Game started successfully");
                                        }
                                        Err(error) => {
                                            LOGGER
                                                .error(format!("Failed to start game: {}", error));
                                        }
                                    }
                                }
                            }
                            RuntimeState::Playing => {
                                if ui.button("⏸ Pause").clicked() {
                                    self.game_runtime.pause();
                                }
                            }
                            RuntimeState::Paused => {
                                if ui.button("▶ Resume").clicked() {
                                    self.game_runtime.resume();
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

            // While playing, the runtime reads input itself (in
            // `update`); otherwise it drives the editor camera
            if self.game_runtime.get_state() != RuntimeState::Playing {
                ctx.input(|input| self.input_handler.handle_input(input));
            }

            // Then handle editor viewport controls only when not playing, and
            // only when the pointer is over the viewport itself (not over a
            // window or popup floating above it)
            if self.game_runtime.get_state() != RuntimeState::Playing
                && ui.rect_contains_pointer(viewport_rect)
                && let Some(cursor_pos) = ui.ctx().input(|i| i.pointer.hover_pos())
            {
                // Editor camera controls
                if self
                    .input_handler
                    .is_mouse_button_pressed(egui::PointerButton::Secondary)
                    || (self
                        .input_handler
                        .is_mouse_button_pressed(egui::PointerButton::Primary)
                        && ui.ctx().input(|i| i.modifiers.alt))
                {
                    ui.ctx().input(|i| {
                        let delta = i.pointer.delta();
                        // Convert screen-space delta to world units
                        let zoom = self.render_engine.camera.zoom.max(0.0001);
                        self.render_engine
                            .camera
                            .move_by(-delta.x / zoom, -delta.y / zoom);
                    });
                }

                // Editor zoom control: zoom towards the cursor, proportionally
                // to the scroll amount (so speed doesn't depend on the frame
                // rate); horizontal scrolling doesn't zoom
                if let Some(scroll_delta) = self.input_handler.get_scroll_delta()
                    && scroll_delta.y != 0.0
                {
                    let zoom_factor = (scroll_delta.y * 0.002).exp();

                    let camera = &mut self.render_engine.camera;
                    let old_zoom = camera.zoom.max(0.0001);
                    let local = cursor_pos - viewport_rect.min;
                    // World point under the cursor before zooming
                    let world_x = local.x / old_zoom + camera.position.0;
                    let world_y = local.y / old_zoom + camera.position.1;

                    camera.zoom_by(zoom_factor);

                    // Keep that world point under the cursor
                    let new_zoom = camera.zoom.max(0.0001);
                    camera.position.0 = world_x - local.x / new_zoom;
                    camera.position.1 = world_y - local.y / new_zoom;
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
                            if let Some(cursor_pos) = ui.ctx().input(|i| i.pointer.hover_pos()) {
                                ui.colored_label(
                                    white,
                                    format!("Cursor: ({:.1}, {:.1})", cursor_pos.x, cursor_pos.y),
                                );
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
    }

    fn render_scene(&mut self, ui: &mut egui::Ui) {
        let content_rect = ui.available_rect_before_wrap();

        // Update render engine with the full viewport dimensions first
        self.render_engine
            .update_viewport_size(content_rect.width(), content_rect.height());
        self.render_engine.set_grid_size(self.gui_state.grid_size);

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
        let Some(active_scene_id) = scene_manager.active_scene else {
            return;
        };
        let Some(active_scene) = scene_manager.get_scene(active_scene_id) else {
            return;
        };
        let render_queue = self.render_engine.render(active_scene);

        let selected_entity = match self.gui_state.scene_panel_selected_item {
            crate::state::ScenePanelSelectedItem::Entity(_, entity_id) => Some(entity_id),
            _ => None,
        };

        let ctx = ui.ctx().clone();
        rust_2d_game_engine::render::paint_sprites(
            ui.painter(),
            content_rect.min,
            &render_queue,
            |id| {
                self.render_engine
                    .get_egui_texture(&ctx, id)
                    .map(|texture| texture.id())
            },
        );

        // Selection highlight, over every sprite
        if let Some(entry) = render_queue
            .iter()
            .find(|entry| Some(entry.entity_id) == selected_entity)
        {
            ui.painter().rect_stroke(
                entry.screen_rect(content_rect.min),
                0.0,
                egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 200, 0)),
                egui::StrokeKind::Outside,
            );
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
        render_queue: &[rust_2d_game_engine::render::RenderQueueEntry],
    ) {
        let response = ui.interact(
            content_rect,
            ui.make_persistent_id("scene_viewport_interaction"),
            egui::Sense::click_and_drag(),
        );

        // A move whose end this view missed (it wasn't shown then) is over
        if !response.dragged() && !response.drag_stopped() && self.viewport_drag.take().is_some() {
            self.gui_state.commit_change();
        }

        // Painting a tilemap takes the place of selecting and dragging. A
        // drag that began before, like the one that selected the tilemap,
        // stays a move until it ends.
        if self.viewport_drag.is_none()
            && let Some(tilemap_id) = self.painted_tilemap(active_scene_id)
        {
            self.paint_tiles(ui, &response, content_rect, active_scene_id, tilemap_id);
            return;
        }
        self.finish_tile_stroke();

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
        if response.clicked()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            match hit_test(pointer) {
                Some(entity_id) => self.gui_state.select_entity(active_scene_id, entity_id),
                None => self.gui_state.clear_selection(),
            }
        }

        // Drag start: grab the entity under the point where the press began
        // (Alt+drag pans instead)
        if response.drag_started_by(egui::PointerButton::Primary)
            && !alt_held
            && let Some(pointer) = ui.input(|i| i.pointer.press_origin())
            && let Some(entity_id) = hit_test(pointer)
        {
            // World position of the cursor and the entity's grab offset
            let camera = &self.render_engine.camera;
            let world_x = (pointer.x - content_rect.min.x) / zoom + camera.position.0;
            let world_y = (pointer.y - content_rect.min.y) / zoom + camera.position.1;

            if let Some(scene_manager) = &self.gui_state.scene_manager
                && let Some(scene) = scene_manager.get_scene(active_scene_id)
                && let Ok(entity) = scene.get_entity(entity_id)
            {
                let offset = (world_x - entity.get_x(), world_y - entity.get_y());
                self.viewport_drag = Some((entity_id, offset));
                // Select what we grab
                self.gui_state.select_entity(active_scene_id, entity_id);
            }
        }

        // Dragging: move the grabbed entity with the cursor
        if response.dragged_by(egui::PointerButton::Primary)
            && !alt_held
            && let (Some((entity_id, offset)), Some(pointer)) =
                (self.viewport_drag, response.interact_pointer_pos())
        {
            let camera = &self.render_engine.camera;
            let world_x = (pointer.x - content_rect.min.x) / zoom + camera.position.0;
            let world_y = (pointer.y - content_rect.min.y) / zoom + camera.position.1;
            let snapping = self.gui_state.snap_to_grid != ui.input(|i| i.modifiers.shift);
            let grid = snapping.then_some(self.gui_state.grid_size);
            let (new_x, new_y) = dragged_position((world_x, world_y), offset, grid);

            if let Some(scene_manager) = &mut self.gui_state.scene_manager
                && let Some(scene) = scene_manager.get_scene_mut(active_scene_id)
                && let Ok(entity) = scene.get_entity_mut(entity_id)
            {
                let _ = entity.set_x(new_x);
                let _ = entity.set_y(new_y);
                // Keep the optional position Vector2 in sync
                if let Ok(pos_attr) = entity.get_attribute_by_name("position") {
                    let pos_id = pos_attr.id;
                    let _ = entity.modify_attribute(
                        pos_id,
                        None,
                        None,
                        Some(rust_2d_game_engine::scene::AttributeValue::Vector2(
                            new_x, new_y,
                        )),
                    );
                }
            }
        }

        // Drag end: persist the move
        if response.drag_stopped_by(egui::PointerButton::Primary)
            && self.viewport_drag.take().is_some()
        {
            self.gui_state.commit_change();
        }
    }

    /// The tilemap being painted: the selected entity, when paint mode is on
    /// and it is a tilemap with a tileset in the scene the viewport shows.
    /// Without a tileset, painted tiles would be invisible, yet solid.
    fn painted_tilemap(&self, active_scene_id: uuid::Uuid) -> Option<uuid::Uuid> {
        let crate::state::SelectedItem::Entity(scene_id, entity_id) = self.gui_state.selected_item
        else {
            return None;
        };
        let entity = self
            .gui_state
            .scene_manager
            .as_ref()?
            .get_scene(scene_id)?
            .get_entity(entity_id)
            .ok()?;
        let has_tileset = entity
            .tilemap
            .as_ref()
            .is_some_and(|tilemap| !tilemap.tileset.as_os_str().is_empty());
        (self.gui_state.painting_tiles && scene_id == active_scene_id && has_tileset)
            .then_some(entity_id)
    }

    /// Paint mode: the left button paints the brush into the cells under
    /// the pointer, one undo step per stroke; Esc ends paint mode.
    fn paint_tiles(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        content_rect: egui::Rect,
        scene_id: uuid::Uuid,
        tilemap_id: uuid::Uuid,
    ) {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.gui_state.painting_tiles = false;
            self.finish_tile_stroke();
            return;
        }
        let camera = self.render_engine.camera;
        let to_world = |pos: egui::Pos2| {
            camera.screen_to_world((pos.x - content_rect.min.x, pos.y - content_rect.min.y))
        };
        let pressing = response.is_pointer_button_down_on()
            && ui.input(|i| i.pointer.primary_down() && !i.modifiers.alt);
        let brush = match self.gui_state.tile_brush {
            crate::state::TileBrush::Tile(tile) => Some(tile),
            crate::state::TileBrush::Eraser => None,
        };
        let Some(entity) = self
            .gui_state
            .scene_manager
            .as_mut()
            .and_then(|manager| manager.get_scene_mut(scene_id))
            .and_then(|scene| scene.get_entity_mut(tilemap_id).ok())
        else {
            return;
        };
        let origin = (entity.get_x(), entity.get_y());
        let name = entity.name.clone();
        let Some(tilemap) = entity.tilemap.as_mut() else {
            return;
        };
        let tile = (tilemap.tile_size.0 as f32, tilemap.tile_size.1 as f32);

        // The cell under the pointer
        if let Some(pointer) = response.hover_pos() {
            let (column, row) = tilemap.cell_at(origin, to_world(pointer));
            let min = camera.world_to_screen((
                origin.0 + column as f32 * tile.0,
                origin.1 + row as f32 * tile.1,
            ));
            let cell = egui::Rect::from_min_size(
                content_rect.min + egui::vec2(min.0, min.1),
                egui::vec2(tile.0 * camera.zoom, tile.1 * camera.zoom),
            );
            ui.painter().rect_stroke(
                cell,
                0.0,
                egui::Stroke::new(1.5, egui::Color32::WHITE),
                egui::StrokeKind::Inside,
            );
        }
        ui.painter().text(
            content_rect.left_top() + egui::vec2(8.0, 36.0),
            egui::Align2::LEFT_TOP,
            format!("🖌 Painting '{}' · Esc to stop", name),
            egui::FontId::proportional(13.0),
            egui::Color32::WHITE,
        );

        if pressing && let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) {
            let cell = tilemap.cell_at(origin, to_world(pointer));
            let stroke = self.tile_stroke.get_or_insert_default();
            // Fill the cells between this frame's and the last one's, so
            // fast strokes have no gaps
            for (column, row) in cells_on_line(stroke.last_cell.unwrap_or(cell), cell) {
                match tilemap.set(column, row, brush) {
                    Ok(changed) => stroke.changed |= changed,
                    Err(e) if !stroke.reported => {
                        LOGGER.warning(format!("Can't paint there: {}", e));
                        stroke.reported = true;
                    }
                    Err(_) => {}
                }
            }
            stroke.last_cell = Some(cell);
        } else {
            self.finish_tile_stroke();
        }
    }

    /// End the stroke being painted: one undo step if it changed anything.
    pub(super) fn finish_tile_stroke(&mut self) {
        if let Some(stroke) = self.tile_stroke.take()
            && stroke.changed
        {
            self.gui_state.commit_change();
        }
    }

    /// Give the runtime a copy of the editor's scenes to play.
    pub(super) fn sync_scene_manager_to_runtime(&mut self) {
        if let Some(scene_manager) = &self.gui_state.scene_manager {
            self.game_runtime.set_scene_manager(scene_manager.clone());
        }
    }
}

/// A tilemap stroke in progress.
#[derive(Default)]
pub(super) struct TileStroke {
    /// The cell painted last
    last_cell: Option<(i32, i32)>,
    /// Whether any cell changed
    changed: bool,
    /// Whether a cell that couldn't be painted was reported
    reported: bool,
}

/// The cells on the straight line from one cell to another, both included.
/// Lines longer than a tilemap can be are reduced to their end.
fn cells_on_line(from: (i32, i32), to: (i32, i32)) -> Vec<(i32, i32)> {
    let (dx, dy) = (
        (to.0 as i64 - from.0 as i64).abs(),
        -(to.1 as i64 - from.1 as i64).abs(),
    );
    if dx > rust_2d_game_engine::scene::MAX_TILEMAP_SIDE as i64
        || -dy > rust_2d_game_engine::scene::MAX_TILEMAP_SIDE as i64
    {
        return vec![to];
    }
    let step = |from: i32, to: i32| if from < to { 1 } else { -1 };
    let (step_x, step_y) = (step(from.0, to.0), step(from.1, to.1));
    let (mut x, mut y) = from;
    let mut error = dx + dy;
    let mut cells = Vec::new();
    loop {
        cells.push((x, y));
        if (x, y) == to {
            return cells;
        }
        let doubled = 2 * error;
        if doubled >= dy {
            error += dy;
            x += step_x;
        }
        if doubled <= dx {
            error += dx;
            y += step_y;
        }
    }
}

/// Where a dragged entity goes: its top-left corner follows the pointer
/// (`pointer` minus the grab `offset`, in world units), rounded to the
/// nearest multiple of `grid` when snapping.
fn dragged_position(pointer: (f32, f32), offset: (f32, f32), grid: Option<f32>) -> (f32, f32) {
    let position = (pointer.0 - offset.0, pointer.1 - offset.1);
    match grid {
        Some(grid) if grid > 0.0 => (
            (position.0 / grid).round() * grid,
            (position.1 / grid).round() * grid,
        ),
        _ => position,
    }
}

#[cfg(test)]
mod tests {
    use super::dragged_position;

    #[test]
    fn test_cells_on_line() {
        use super::cells_on_line;
        assert_eq!(cells_on_line((0, 0), (0, 0)), vec![(0, 0)]);
        assert_eq!(
            cells_on_line((0, 0), (3, 0)),
            vec![(0, 0), (1, 0), (2, 0), (3, 0)]
        );
        assert_eq!(cells_on_line((2, 2), (0, 0)), vec![(2, 2), (1, 1), (0, 0)]);
        let steep = cells_on_line((0, 0), (1, 4));
        assert_eq!((steep.len(), steep[0], steep[4]), (5, (0, 0), (1, 4)));
        assert_eq!(
            cells_on_line((0, 0), (i32::MAX, 0)),
            vec![(i32::MAX, 0)],
            "too long"
        );
    }

    #[test]
    fn test_dragged_position_snaps_to_the_grid() {
        assert_eq!(
            dragged_position((110.0, 75.0), (10.0, 5.0), None),
            (100.0, 70.0)
        );
        assert_eq!(
            dragged_position((110.0, 75.0), (10.0, 5.0), Some(32.0)),
            (96.0, 64.0)
        );
        assert_eq!(
            dragged_position((-20.0, 9.0), (0.0, 0.0), Some(16.0)),
            (-16.0, 16.0)
        );
        assert_eq!(
            dragged_position((5.0, 5.0), (0.0, 0.0), Some(0.0)),
            (5.0, 5.0)
        );
    }
}
