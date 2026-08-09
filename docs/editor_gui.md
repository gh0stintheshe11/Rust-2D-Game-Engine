# Editor GUI (`src/engine_gui/` + `src/gui/`)

The eframe application shell: one window containing the menu bar, the scene
hierarchy + file browser (left), the inspector (right), the console (bottom),
and a center area that is either the **game viewport** (Viewer tab) or the
**script editor** (Editor tab).

## Layout & modules

| Piece | Module | Role |
|---|---|---|
| App shell | `engine_gui/mod.rs` | Panel layout, tab switching, exit flow, undo/redo shortcuts, viewport interaction |
| Script editor | `engine_gui/script_editor.rs` | Code editing (see below) |
| Scene hierarchy | `gui/scene_hierarchy/` | Scene/entity tree, create/rename/delete popups, asset attach/detach |
| File browser | `gui/file_system.rs` | Cached project tree (2s refresh + manual ⟳), file selection/deletion |
| Inspector | `gui/inspector.rs` | Entity attributes (edit/add/delete), file previews, script snippet insertion |
| Menus | `gui/menus/` | File (new/open/save/exit), Edit (undo/redo), View (panels/theme), Import, Project (build) |
| Shared state | `gui/gui_state.rs` | Selection, project state, undo stack, cross-panel request channels |

## Cross-panel collaboration

Panels talk through request channels on `GuiState`, consumed by the shell
each frame:

- `open_script_request: Option<PathBuf>` — set by the file browser, the
  hierarchy's script row, or the inspector's Edit Script button; the shell
  loads the file into the editor and switches to the Editor tab.
- `script_insert_request: Option<String>` — set by the inspector when an
  **attribute name is clicked**: `get_attribute(scene_id, entity_id, "…")`
  is inserted at the editor cursor.

## Viewport (Viewer tab, not playing)

- **Left-click**: select the topmost entity under the cursor (gold outline;
  hierarchy/inspector follow). Click empty space to deselect.
- **Left-drag**: move the grabbed entity in world space (zoom-corrected);
  the move persists on release and is one undo step.
- **Right-drag / Alt+drag**: pan the camera (world-unit speed at any zoom).
- **Scroll**: zoom towards the cursor.
- Grid lines are world-locked with power-of-two adaptive spacing.

While playing, the same area renders the game (`GameRuntime::update`);
Paused/Ended keep the freeze frame visible.

## Script editor (Editor tab)

- Lua syntax highlighting (syntect), **line numbers**, Ln/Col indicator.
- **Live syntax checking**: the buffer is parsed by Lua on every change;
  the first error is shown with its line number above the code.
- **Engine API palette** (📖 API toggle): every scripting binding, grouped,
  hover for signature + doc, click to insert at the cursor.
- **Attribute insertion**: click an attribute name in the inspector to
  insert a `get_attribute(...)` call at the cursor.
- Empty files offer a one-click `init`/`update`/`on_collision` template.
- Saving: **Ctrl+S**, focus loss, switching files/tabs, or pressing Play
  (scripts hot-reload from disk). Unsaved changes show a ● marker.

## Persistence & undo

Every completed mutation (create/rename/delete of scenes and entities,
attribute edits, asset attach/detach, viewport drags) saves the project
**and commits an undo snapshot**. `Ctrl+Z` / `Ctrl+Y` (when no text field is
focused) or Edit menu. History: 50 states, reset on project open.

Destructive operations (scene/entity/file delete, exit with a project open)
ask for confirmation first.

## Known limitations / TODO

- No multi-select, no entity duplication, no drag-drop of assets onto
  entities, no gizmos (rotation/scale handles).
- Console lacks clear/copy buttons.
- `rfd` file dialogs block the UI thread while open (native modality).
- The editor and the game runtime still hold separate `RenderEngine` clones
  (duplicate texture caches, camera synced per frame).
