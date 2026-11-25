# Changelog

All notable changes to this project are documented here. The format is based
on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/) (while on 0.x, minor
versions may contain breaking changes).

## [Unreleased]

## [0.8.0] - 2026-11-25

Game feel and polish: a camera that follows and shakes, transitions
between scenes, audio that fades, music that crossfades and volume groups
for settings menus, and tilemaps that fill rectangles and regions and
update their colliders a cell at a time (a changed tile costs 0.3 µs on a
map of any size). Hilltop Hop uses them all, and now has music. When
upgrading, note that `audio::PlayOptions` has new fields and is no longer
`Copy`, and that `LuaScripting::take_scene_change_request` returns a
`SceneChange`.

### Added

- The camera follows an entity: `camera_follow(entity_id, options)` keeps
  the scene's camera on it (on its physics body's middle), with `smoothing`
  (how far behind it lags, the same at any frame rate), a dead zone, an
  offset and `bounds` the frame stays inside, until the scene changes or
  `camera_follow(nil)`. `camera_shake(strength, seconds)` shakes it, fading
  out; the shake is added to the camera and taken off again, so it never
  builds up or fights a script moving the camera. The camera moves before
  the frame's updates, so backgrounds placed by it don't lag. `get_camera`
  returns a scene's camera. In Rust: `render::CameraRig`, and
  `PhysicsEngine::body_center`.
- Scene transitions: `change_scene(scene, {transition = "fade", seconds =
  0.5, color = "#000000"})` covers the camera frame with a color, changes
  the scene once it's covered, and uncovers the new one: a fade, or a wipe
  in one of four directions. Changes asked for while the frame is being
  covered are ignored. In Rust: `GameRuntime::change_scene_with`,
  `transition_coverage` and `render::Transition`.
- Audio fades, music and volume groups. Volumes fade on the audio thread,
  sample by sample, so fades are smooth at any frame rate:
  `play_sound(path, {fade_in = 2})`, `fade_sound(play_id, volume,
  seconds)`, and `stop_sound(play_id, seconds)` fading out before it stops
  (`AudioEngine::fade`, `stop_with_fade`). `play_music(path, {crossfade =
  1})` makes a file the music, which loops and crossfades into the next
  track, or plays on when it's the same file; `stop_music(seconds)` fades
  it out. Sounds belong to volume groups (`"sfx"` by default, `"music"` for
  music, or any name), which `set_group_volume` scales: settings menus.
  Mixing costs about 0.8 ns more per sample of each sound.
  `GameRuntime::audio` lends out the game's audio engine.
- Rectangle and fill tools for tilemaps. In the editor's paint mode, R
  drags out a rectangle, filled once the button is released, and F fills
  the cell clicked and every cell connected to it holding the same (a paint
  bucket), within the tiles' bounds; B is the brush. Each is one undo step.
  Scripts have `fill_tiles` and `flood_fill_tiles`, and Rust code
  `Tilemap::fill_rect` and `Tilemap::flood_fill`, which return the cells
  that changed.

### Changed

- Hilltop Hop's camera follows the player with `camera_follow` (a moment
  behind, within the level) and shakes when the player gets hurt or breaks
  a brick; both demos go between scenes through transitions. Hilltop Hop
  has music: a title track and a level track that crossfade, the level's
  quieter under the end screens, and a Music on / off setting.
- `LuaScripting::take_scene_change_request` returns a `SceneChange` (the
  scene and its transition) instead of the scene's id.
- Changing tiles while playing updates the tilemap's colliders cell by
  cell (`PhysicsEngine::update_tilemap_cells`) instead of rebuilding them:
  a changed tile costs about 0.3 µs, including the next physics step, on a
  map of any size, against 90 µs for Hilltop Hop's level and 4.7 ms for a
  map of 100,000 tiles. Tilemap colliders have no density (their bodies are
  fixed), which kept rapier from going through every tile after a change.
- `audio::PlayOptions` has `group` and `fade_in` fields and is no longer
  `Copy` (breaking for Rust code that constructs it without
  `..PlayOptions::default()`). Setting a sound's volume gets there over 5
  ms instead of jumping, so it doesn't click.

## [0.7.0] - 2026-11-25

Less code for everyday game logic: input actions, named controls bound to
keys and gamepads in the editor and rebound by players; timers and tweens
for scripts; and an `on_destroy` hook. Both demos use them, and Star
Catcher plays with a gamepad. When upgrading, Rust code that constructs
`ProjectMetadata` needs its new `input_actions` field.

### Added

- Input actions: named controls such as `jump` or `left`, each bound to
  keys, mouse buttons, gamepad buttons and stick or trigger directions, any
  gamepad or a given one (`key:Space`, `pad:a`, `pad2:start`,
  `pad:left_x-`). Scripts read them with `is_action_pressed`,
  `is_action_just_pressed`, `is_action_just_released`,
  `get_action_strength` (sticks hold an action part way) and
  `get_action_axis("left", "right")`, rebind them with
  `set_action_bindings` until the game stops, and build "press a key"
  menus with `get_just_pressed_binding`. A stick or trigger presses an
  action from a threshold of 0.5, or the action's own. Actions are kept in
  `project.epm` (`ProjectMetadata::input_actions`, `input::InputActions`),
  which builds now copy next to the game; new projects start with `left`,
  `right`, `up`, `down`, `jump` and `pause`. In the editor, Project →
  Input Actions edits them: press a key to bind it, or pick a mouse button
  or gamepad control from a menu.
- Timers and tweens for scripts: `after(seconds, fn)` and `every(seconds,
  fn)` call back on game time, `tween(entity_id, {x = 300, opacity = 0},
  seconds, options)` moves numeric attributes with an ease (quad, cubic,
  sine, back, elastic, bounce), a delay, a loop (restart or ping-pong) and
  an `on_done` callback, and `cancel(id)` stops either. They belong to the
  entity whose script made them, and stop when it's removed or the scene
  changes. `LuaScripting::timer_count` says how many are waiting.
- An `on_destroy(scene_id, entity_id)` hook: `remove_entity` runs it before
  the entity goes, so it can still read where the entity was.

### Changed

- Both demos read their controls through input actions, and Star Catcher
  plays with a gamepad too. Hilltop Hop's gems bob with a tween and sparkle
  from `on_destroy`, and a checkpoint's flag hops when reached.
- `ProjectMetadata` has an `input_actions` field (breaking for Rust code
  that constructs it).

## [0.6.0] - 2026-11-24

Platformer building blocks: tilemaps with one-way platforms and animated
tiles, particles, game UI, gamepads and save data, shown in a second demo
game, Hilltop Hop; faster frames; and in the editor, copy and paste, grid
snapping and an Add Feature menu. When upgrading, note that
`get_mouse_position()` now returns world coordinates, and that building on
Linux needs `libudev-dev`.

### Added

- Tilemaps: an entity can carry a grid of tiles drawn from a tileset image
  (`Entity::tilemap`, a boxed `scene::Tilemap`). Only the visible tiles are
  drawn, in one mesh; with `has_collision` the tiles form one voxel collider,
  so bodies slide along rows of tiles without catching on their edges.
  Scripts read and change tiles with `get_tile`, `set_tile` (which rebuilds
  the collider) and `get_tile_cell`. Scene files store a map as readable
  rows. In the editor, Create → Tilemap makes one, and its inspector section
  picks the tileset and offers a palette and a paint mode: left-drag paints
  in the viewport, one undo step per stroke.
- Tiles can be one-way platforms or passable: each tile of a tileset
  collides as solid (the default), as a platform that bodies land on but
  jump up through and walk through sideways, or not at all, for decoration
  (`Tilemap::set_tile_collision`; the editor sets it below the palette).
  Bodies passing through a platform don't collide with it, and
  `drop_through_platforms(entity_id)` lets a body fall through the
  platform it stands on (pressing down, in a platformer).
- Animated tiles: a tile of a tileset can show other tiles in turn, for
  water, torches or grass in the wind (`Tilemap::set_tile_animation`; the
  editor sets the frames and speed below the palette, and previews the
  selected tilemap). Cells keep their tile for collisions and scripts.
- Particles: an entity with `particle_*` attributes (Add Feature → Effects →
  Particles) gives off small sprites for sparks, smoke, dust or rain, with a
  rate, lifetime, velocity and spread, gravity, and size and color changing
  over their life; its image, if any, is their texture. Scripts fire bursts
  with `emit_particles`, which play out even when the emitter is removed
  right after. Up to 20,000 particles are simulated and drawn in a few
  batched meshes (10,000 cost about 0.35 ms per frame). The editor marks
  emitters in the viewport and previews the selected one.
- Game UI: scripts draw rectangles (`draw_rect`: panels, health bars),
  images (`draw_image`: icons, lives) and buttons over the game.
  `button(label, x, y, width, height)` returns true on the frame it's
  clicked, pressed and released on it, and shows hover and press states.
  Buttons work with a keyboard or a gamepad too: the arrow keys, Tab, the
  d-pad or the left stick move the focus among them, Enter or gamepad A
  press the focused one, and a `default` button has the focus when the
  buttons appear. Like `draw_text` they're laid out in camera frame
  pixels, and later drawings cover earlier ones.
  `InputHandler::get_mouse_press_screen_pos` tells where the left button
  went down.
- Gamepads: scripts read buttons (`is_gamepad_pressed`,
  `is_gamepad_just_pressed`, `is_gamepad_just_released`, with Xbox names
  like `"a"`, `"lb"` or `"up"`, and PlayStation ones) and axes
  (`get_gamepad_axis`: sticks with a round dead zone and +y down like the
  world, triggers from 0 to 1), on any pad or on pad n, and count pads with
  `get_gamepad_count`. Exported games and playtests in the editor read
  the system's gamepads through gilrs (`GameRuntime::set_gamepads`), and
  `GameRuntime::queue_gamepad_events` feeds gamepad events, e.g. from
  tests. Building on Linux now needs `libudev-dev`.
- Hilltop Hop, a second demo game: a platformer on a tilemap level with
  one-way platforms, breakable bricks, spikes, gems, checkpoints, swaying
  grass, particles, a title menu and HUD made with the game UI functions,
  and gamepad support (`demo/hilltop_hop`, `cargo run -p hilltop_hop`).
- `get_velocity(entity_id)` returns a body's velocity to scripts, so they
  can change one direction and keep the other (a platformer's run speed
  under gravity).
- Scripts see releases and mouse clicks: `is_key_just_released`,
  `is_mouse_just_pressed` and `is_mouse_just_released`.
- `get_mouse_screen_position()` returns the pointer in the camera frame's
  pixels, the coordinates `draw_text` uses, e.g. for clickable menus.
- `InputHandler::handle_input_with` takes in only the input an
  `InputAccess` allows, `Camera::screen_to_world` inverts `world_to_screen`,
  and `GameRuntime::input` returns the game's input handler.
- Editor: Ctrl+D duplicates the selected entity, Ctrl+C / Ctrl+V copy and
  paste it (through the system clipboard, so also between projects), and
  Delete or Backspace deletes it. The hierarchy's context menus have
  Duplicate, Copy and Paste Entity.
- Editor: dragged entities snap to the grid (hold Shift to move freely), and
  View → Grid sets the grid size and turns snapping off.
- Editor: the inspector's Add Feature menu adds a feature's attributes with
  working values: physics bodies, trigger zones, collision layers, sprite
  effects, sprite sheet animation and more. The engine provides them as
  `attribute_presets()` with `Entity::apply_preset` and `has_preset`.
- Save data: scripts keep values between runs of the game with
  `save_data(key, value)` and `load_data(key)`. Exported games store them in
  the user's data folder, games played in the editor in the project's
  `saves/` folder (ignored by version control in new projects), which
  Project → Clear Save Data empties (`SaveData`,
  `GameRuntime::set_save_data`).
- `ProjectManager::make_entity_paths_relative` and `resolve_entity_paths`
  convert a single entity's paths, and `RenderEngine::set_grid_size` sets
  the editor grid's cell size.

### Changed

- Faster frames, measured with the new `cargo bench --bench frame`: calling
  a script's hooks costs about a quarter of what it did (entity and scene ids
  are passed as cached Lua strings, script files are looked up once per
  frame, and only scripts with collision hooks track contacts), and drawing
  sprites about a fifth (consecutive sprites with the same texture are
  painted as one mesh, texture ids are cached, and looking up a missing
  attribute no longer allocates). `render::paint_sprites` paints a render
  queue that way, and `Entity::attribute` looks attributes up without
  allocating.
- The editor draws the selection outline over every sprite instead of right
  above the selected one.
- `get_mouse_position()` returns the pointer in world coordinates, through
  the camera, instead of window coordinates, which differed between the
  editor and exported games (breaking for scripts that relied on window
  coordinates; `get_mouse_screen_position()` gives camera frame pixels). In
  an exported game whose camera frame starts at (0, 0) and fills the window
  at its own size, the values are unchanged.
- The Star Catcher demo saves its best score between runs.
- `LuaScripting::hud()` returns everything scripts drew over the game this
  frame (`HudItem`s); `hud_texts()` returns copies of the texts among them,
  instead of a borrow of a list of texts (breaking for Rust code that used
  the borrow).
- `Entity` has a `tilemap` field, `RenderQueueEntry` a `quads` field and
  `ColliderOutline` a `one_way` field (breaking for Rust code that
  constructs them).

### Fixed

- A key or mouse button pressed and released between two frames was missed;
  it now counts as just pressed (and just released) in the next frame.
- While playing in the editor, typing in a text field and clicking the play
  buttons or panels reached the game as key presses and clicks.
- `cargo run -p star_catcher` from the repository root, as the demo's
  README says, couldn't find the game's project; games started with
  `cargo run` now also look in their package's folder.

## [0.5.0] - 2026-11-24

Games with several scenes and prefabs, and trigger zones, collision layers
and raycasts for physics.

### Added

- Trigger zones: colliders with `is_sensor = true` report overlaps (through
  `get_colliding_entities` and `on_collision`) without blocking anything.
- Collision layers: `collision_layer` (1 to 32) and `collides_with` (e.g.
  `"1,3"`) decide which colliders interact.
- Raycasts: `PhysicsEngine::raycast`, and `raycast(x1, y1, x2, y2,
  exclude_id)` in scripts, return the first collider a segment hits.
- An `on_collision_end(scene_id, entity_id, other_id)` script hook, called
  when a contact or overlap ends, including when the other entity was
  removed.
- Scene switching: scripts call `change_scene("level 2")` (Rust code:
  `GameRuntime::switch_scene`). The scene starts over as it was when the game
  started; `script_state` and playing sounds carry over.
- Prefabs: `spawn(scene_id, "prefabs/bullet", x, y)` copies an entity, with
  its attributes, images, script and physics body, as it was when the game
  started. A scene that never plays works as a prefab library.
  `find_entity(scene_id, name)` returns an entity's id by name.

### Changed

- `ColliderOutline` has an `is_sensor` field (breaking for Rust code that
  constructs outlines), and collider outlines draw sensors in green.
- The Star Catcher demo starts on a title screen that shows the best score,
  and a game over is a screen of its own: Space plays again, Esc returns to
  the title. Stars and rocks are prefabs in a `prefabs` scene, on a collision
  layer of their own so they no longer bump into each other.

### Fixed

- While playing, the editor's red camera rectangle stayed where the camera
  was before Play. It now follows the game's camera as scripts move it, and
  shows the camera of the scene the game switched to.

## [0.4.0] - 2026-11-24

Text, sprite animation and audio volume for games, and exported games no
longer show debug outlines.

### Added

- Text: `draw_text(text, x, y, {size, color, align})` draws text over the
  game (scores, timers, messages). It is laid out in the camera frame's
  pixels, so it scales with the window like the sprites.
- Sprite sheets and animation. An entity with a `frame_size` shows one frame
  of its image (`frame`); `animation = "0-3"` (with `animation_fps` and
  `animation_loop`) plays frames while the game runs, and scripts call
  `play_animation(scene_id, entity_id, "0-3", {fps, loop, restart})`.
  Physics bodies are one frame big.
- Sprites can be mirrored (`flip_x`, `flip_y`), faded (`opacity`) and tinted
  (`tint = "#rrggbb"`) through attributes, in the editor and in games.
- Audio volume and looping. Scripts call
  `play_sound(path, {volume = 0.5, loop = true})`, `set_sound_volume` and
  `set_master_volume`; Rust code uses `AudioEngine::play_sound_with`,
  `set_volume` and `set_master_volume`.
- `AudioEngine::offline` mixes sounds into a sample stream instead of an audio
  device, so playback can be tested on machines without one.
- `Entity::get_bool`, `get_float`, `get_string`, `get_vector2` and
  `set_or_create_attribute` read and write optional attributes by name.

### Changed

- `render::paint_sprite` takes the texture rectangle and tint color to draw
  with, and `RenderQueueEntry` has `uv` and `tint` fields (breaking for Rust
  code that paints sprites itself).
- The Star Catcher demo shows its score as text instead of a row of small
  stars.

### Fixed

- Exported games drew the physics collider outlines, a debugging aid, over
  the game. They are now off unless enabled (`GameRuntime::set_show_colliders`);
  the editor shows them while playing, and View → Collider Outlines hides them.
- Import Sound and the inspector's audio preview accept FLAC files, which the
  engine could already play.

## [0.3.0] - 2026-11-24

The first release as an open-source project, after a full audit of the code.

### Changed (breaking)

- The repository is a cargo workspace: the engine library
  `rust-2d-game-engine` at the root, the editor `rust-2d-game-editor` in
  `editor/` (the editor binary was called `rust-2d-game-engine` before), and
  the demo in `demo/`. Exported games depend on the engine only.
- Engine modules are renamed: `ecs` → `scene`, `physics_engine` → `physics`,
  `render_engine` → `render`, `lua_scripting` → `scripting`, `game_runtime` →
  `runtime`, `audio_engine` → `audio`, `input_handler` → `input`,
  `project_manager` → `project`.
- A game's `main.rs` is a single call to `rust_2d_game_engine::player::run`.
- Physics bodies are centered on their sprite and rotating bodies write their
  angle to the `rotation` attribute. Colliders default to rectangles (the
  "near-square sprites become circles" rule is gone). `apply_force` acts for
  the frame it is called in, independent of the frame rate.
- `GameRuntime` has explicit `run` / `pause` / `resume` / `reset` / `restart`
  and a headless `tick`; the `Game` trait and `InputContext` are removed.
- Physics API: `(f32, f32)` tuples instead of rapier vectors, ids by value,
  `collider_outlines` instead of `get_collider_data`.
- Scripts run in a sandbox: no `io`, `package`/`require`, `debug`, or
  `os.execute`; `os` keeps its clock functions. A hook call is aborted after 1
  second.
- `keys_pressed` names: mouse buttons are `MouseLeft` / `MouseRight` /
  `MouseMiddle`; `Cmd` is reported only for the macOS Command key.
- `project.epm` no longer stores the project's absolute path. Only
  `project.epm` is required to open a project.
- New attributes added in the inspector keep their name (no `metadata_`
  prefix) and must be identifiers.
- Minimum supported Rust version: 1.95. Rust edition 2024.

### Added

- Star Catcher, an original demo game with generated assets, replacing the
  Flappy Bird demo.
- Exported games scale the scene's camera frame to the window (letterboxed),
  show startup errors, and restart with Enter or R after game over.
- Script bindings: `set_angular_velocity`, `create_attribute_string`,
  `create_attribute_int`, and the `delta_time` global; `print` writes to the
  editor console.
- Hot reload keeps a script's variables; a script with errors keeps its last
  working version running.
- Gravity fields (`creates_gravity`) with a `gravity_strength` attribute.
- Editor: console Clear button, repeat counts for repeated messages, macOS
  Cmd shortcuts, Ctrl+Shift+Z redo, warnings before deleting files entities
  use.
- Projects created by downloaded editor releases depend on the matching engine
  release from GitHub.
- Builds include the engine's LICENSE and NOTICE next to the game.
- CI on Linux, Windows and macOS, plus an MSRV check.

### Fixed

- Click-to-select and drag-to-move in the viewport didn't work in any project
  (scenes were keyed by a different id than their own).
- Entering `inf` or `nan` in the inspector made the project unopenable.
- The inspector panicked on non-ASCII attribute names, and the editor crashed
  when previewing images larger than the GPU texture limit.
- Inspector fields could write stale values back after an undo; each edit was
  recorded twice in the undo history; Set Active wasn't saved.
- Re-opening the open script discarded unsaved edits; opening another project
  left the old game running.
- Lua `remove_entity` left the physics body behind as an invisible obstacle,
  and one removed entity could stop other entities from moving.
- Torque accumulated forever; `set_gravity` carried over into the next Play;
  `set_x` / `set_position` were undone by the physics step for physics
  entities.
- A script could crash the editor through `set_attribute` with a table whose
  metatable called back into the engine; `stop_sound(nil)` raised an error.
- Saves could leave truncated files after a crash (they are now atomic), and
  projects cloned from git failed to open when an asset folder was empty.
- Generated projects failed to build for folder names starting with a digit
  or containing braces or quotes.
- Many smaller issues listed in the commit history.

### Removed

- Unused dependencies `rayon` and `sha2`, and dead code surfaced by the
  workspace split.
- Third-party assets of unclear license (the Flappy Bird demo, test audio).

## [0.2.0] - 2026-08-09

Renovation of the 2024 course project: rewritten Lua scripting core, physics
and runtime fixes, GPU texture caching, undo/redo, viewport picking and
dragging, script editor improvements, CI and multi-platform releases.

[Unreleased]: https://github.com/gh0stintheshe11/Rust-2D-Game-Engine/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/gh0stintheshe11/Rust-2D-Game-Engine/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/gh0stintheshe11/Rust-2D-Game-Engine/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/gh0stintheshe11/Rust-2D-Game-Engine/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/gh0stintheshe11/Rust-2D-Game-Engine/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/gh0stintheshe11/Rust-2D-Game-Engine/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/gh0stintheshe11/Rust-2D-Game-Engine/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/gh0stintheshe11/Rust-2D-Game-Engine/releases/tag/v0.2.0
