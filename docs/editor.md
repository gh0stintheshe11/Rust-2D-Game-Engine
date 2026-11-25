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
| Menus | `menu_bar.rs`, `menus/` | File, Edit (undo/redo), View, Import, Project (build, clear save data) |
| State | `state.rs` | Shared state: open project, selection, undo history, requests; duplicating and pasting entities |
| Clipboard | `clipboard.rs` | Entities as clipboard text, for pasting between windows and projects |
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
- **Entities**: Ctrl+D duplicates the selected entity, Ctrl+C / Ctrl+V copy it
  to the system clipboard and paste it into the selected scene (or the active
  one), and Delete or Backspace deletes it after confirmation. The entity's
  context menu has Duplicate and Copy, the scene's has Paste Entity (the last
  entity copied in this editor). A copy keeps the name, attributes and assets
  and gets a new id; when it would exactly cover an entity of the same name,
  it moves 16 px right and down, so repeated duplicates cascade. The clipboard
  holds JSON with project-relative paths (see [project.md](project.md)), so
  entities can be pasted into another project that has the same assets. The
  shortcuts apply when no text field has focus and the game isn't playing.
- **Tilemaps**: Create → Tilemap adds an entity with an empty tilemap of
  32 px tiles and `has_collision` (a solid ground; turn it off for
  decoration). Its inspector section picks the tileset among the project's
  images, sets the tile size, counts the tiles, and has a palette of the
  tileset's tiles (the first 512) plus an Eraser. Below the palette, the
  brush's tile is set to collide as solid, as a one-way platform or not at
  all (passable), and animated: its frames typed like a sprite animation's
  (`5,9` or `5-8`, applied with Enter, emptied to stop) and their speed. In
  the palette, one-way tiles have a cyan top edge, passable ones a dashed
  border and animated ones an orange triangle; the selected tilemap plays
  its animations in the viewport. Once the tilemap has a
  tileset (tiles painted without one would be invisible, yet solid),
  🖌 Paint turns on paint mode for the selected tilemap: left-drag in the
  viewport paints the chosen tile (or erases) into the cells under the
  pointer, filling the cells between two frames' positions; each stroke is
  one undo step. The cell under the pointer is outlined, selecting and
  dragging entities is off (a drag that began before, like the one that
  selected the tilemap, still moves it), right-drag still pans, and Esc ends
  paint mode. See [scene.md](scene.md#tilemaps).
- **Particles**: an emitter (see [render.md](render.md#particles)) shows as
  an orange ring in the viewport, where it can be clicked and dragged; while
  it's selected, it previews its particles, and the viewport keeps
  redrawing as long as particles are alive.
- **Presets**: the inspector's Add Feature menu adds the attributes an engine
  feature needs, with working values: physics (physics, static or kinematic
  body, trigger zone, collision layers, collider size, circle collider,
  gravity scale, gravity field), sprites (scale, rotation, mirror, opacity,
  tint), animation (sprite sheet, animation) and effects (particles).
  Attributes the entity has
  keep their values; presets it has entirely are greyed out (see
  [scene.md](scene.md)).

Edits made while the game is playing change the editor's scenes, not the
running game (Play gives the game its own copy). Undo works while playing.

What the game saves with `save_data` while playing in the editor goes to the
project's `saves/save.json`, not where the exported game keeps its saves (see
[save.md](save.md)). Project → Clear Save Data deletes it.

## Viewport (Viewer tab)

- **Left-click**: select the topmost entity under the cursor (gold outline);
  click empty space to deselect.
- **Left-drag**: move the entity under the press point; one undo step. Its
  position snaps to the grid (View → Grid → Snap to Grid); holding Shift
  inverts snapping.
- **Right-drag / Alt+drag**: pan. **Scroll**: zoom towards the cursor.
- Camera input only applies while the pointer is over the viewport itself,
  not over a window floating above it.
- The grid is world-locked. Its cell size is set in View → Grid (8 to 128
  px, 32 by default); while zoomed out, lines are drawn every 2, 4, …
  cells. The red rectangle is the frame of the scene's default camera.

While playing, the viewport shows the game through the editor's camera, with
collider outlines drawn over it (View → Collider Outlines turns them off).
The game gets the keyboard unless an editor text field has focus (clicking the
game takes it back), and mouse clicks and scrolling only over the game itself,
not over panels or the play buttons. The red rectangle then shows what the
exported game frames: the camera of the scene the game is in, where the game's
scripts moved it. Paused and Ended keep the last frame. The game only advances
while the Viewer tab is shown.

## Script editor (Editor tab)

- Lua syntax highlighting, line numbers, Ln/Col, live syntax checking.
- Engine API palette (📖 API): every binding with its signature and doc; click
  to insert at the cursor.
- Empty files offer a template with `init`, `update`, `on_collision` and
  `on_collision_end`.
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

- No multi-select, drag-and-drop of assets, or gizmos (rotation/scale
  handles).
- Tilemaps are painted cell by cell: there are no rectangle or flood fills,
  brushes of several tiles, or automatic tile borders (auto-tiling).
- Grid settings (snapping, size) aren't saved: every session starts with
  snapping on and 32 px cells.
- `rfd` file dialogs block the UI while open.
- The editor and the runtime each own a `RenderEngine`, so images are decoded
  and uploaded twice.
- The file tree is rescanned on the UI thread every 2 seconds.
