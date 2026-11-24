# Editor (`editor/`)

The `rust-2d-game-editor` binary: an eframe application with the menu bar,
the scene hierarchy and file browser (left), the inspector (right), the
console (bottom), and a center area that is either the **viewport** (Viewer
tab) or the **script editor** (Editor tab).

## Modules

| Piece | Module | Role |
|---|---|---|
| App shell | `app/mod.rs` | Layout, shortcuts, cross-panel requests, project switching |
| Viewport | `app/viewport.rs` | Scene view, picking and dragging, editor camera, play controls |
| Script editor | `app/script_editor.rs` | Lua editing, script buffer open/save |
| Console | `app/console.rs` | Log view with level filter, Debug tab (build output), Clear |
| Exit | `app/exit.rs` | Close interception and the unsaved-changes prompt |
| Scene hierarchy | `panels/hierarchy/` | Scene/entity tree, create/rename/delete popups, asset attach/detach |
| File browser | `panels/files.rs` | Project tree (rescanned every 2 s, ⟳ to refresh), file selection/deletion |
| Inspector | `panels/inspector.rs` | Attributes (edit/add/delete), file previews, snippet insertion |
| Menus | `menu_bar.rs`, `menus/` | File, Edit (undo/redo), View, Import, Project (build) |
| State | `state.rs` | Shared state: open project, selection, undo history, requests |
| Project operations | `project/` | Scaffolding, asset import, builds (see [project.md](project.md)) |

## How the panels cooperate

Panels never call each other. They change `GuiState` or leave a request in it,
and the app shell handles requests after drawing the frame:

- `open_script_request` — open a script in the editor (files panel,
  hierarchy, inspector's Edit Script button). Opening the file that's already
  open just switches to the Editor tab; switching files saves the current one
  first, and if that fails the current file stays open.
- `script_insert_request` — insert a snippet at the editor cursor (clicking an
  attribute name inserts `get_attribute(scene_id, entity_id, "…")`).
- `save_requested` / `build_requested` — File > Save Project and Project >
  Build save the script buffer before saving or building.
- `deleted_file` — closes the script buffer if its file was deleted.
- `project_generation` — opening or creating a project increments it; the
  shell then saves and closes the script buffer, resets the running game,
  clears caches, selection, popups and the inspector.

## Editing, saving and undo

Every completed change (create/rename/delete of scenes and entities, attribute
edits, asset attach/detach, viewport drags, Set Active) goes through
`GuiState::commit_change`: the project is **saved** and an **undo step** is
recorded, unless nothing actually changed. Save failures are reported in the
console.

- Undo: Ctrl+Z (Cmd+Z on macOS). Redo: Ctrl+Y or Ctrl+Shift+Z. They apply
  when no text field has focus, or through the Edit menu. The history keeps 50
  states and starts over when a project is opened.
- Inspector fields always show the current value. Text typed into a field is
  applied when the field loses focus (or Enter), when another item is
  selected, and only if it parses as the attribute's type and differs from the
  current value. Infinite and NaN numbers are rejected.
- New attributes need a name made of ASCII letters, digits and underscores
  that doesn't start with a digit, so it can be used from scripts as is.
- Deleting a scene, entity or file asks for confirmation; deleting a file that
  entities still use lists them.

Edits made while the game is playing change the editor's scenes, not the
running game (Play gives the game its own copy). Undo works while playing.

## Viewport (Viewer tab)

- **Left-click**: select the topmost entity under the cursor (gold outline);
  click empty space to deselect.
- **Left-drag**: move the entity under the press point; one undo step.
- **Right-drag / Alt+drag**: pan. **Scroll**: zoom towards the cursor.
- Camera input only applies while the pointer is over the viewport itself,
  not over a window floating above it.
- The grid is world-locked with power-of-two spacing; the red rectangle is the
  frame of the scene's default camera.

While playing, the viewport shows the game through the editor's camera (the
red rectangle shows what the exported game will frame). Paused and Ended keep
the last frame. The game only advances while the Viewer tab is shown.

## Script editor (Editor tab)

- Lua syntax highlighting, line numbers, Ln/Col, live syntax checking.
- Engine API palette (📖 API): every binding with its signature and doc; click
  to insert at the cursor.
- Empty files offer a template with `init`, `update` and `on_collision`.
- Saving: Ctrl+S (Cmd+S), focus loss, switching files or tabs, Play, Save
  Project, Build. Unsaved changes show a ● marker.

## Exit

Scene edits are saved as they happen, so exiting only asks for confirmation
when the script editor has unsaved changes (Save & Exit / Discard & Exit /
Cancel). If saving fails, the editor stays open. The prompt also appears when
the window is closed while minimized.

## Testing

`app/mod.rs` has headless tests that run the whole editor UI through
`egui::Context::run_ui` with a real project, catching panics in panels without
a window.

## Known limitations

- No multi-select, entity duplication, drag-and-drop of assets, or gizmos
  (rotation/scale handles).
- `rfd` file dialogs block the UI while open.
- The editor and the runtime each own a `RenderEngine`, so images are decoded
  and uploaded twice.
- The file tree is rescanned on the UI thread every 2 seconds.
