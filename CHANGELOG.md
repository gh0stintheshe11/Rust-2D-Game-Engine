# Changelog

All notable changes to this project are documented here. The format is based
on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/) (while on 0.x, minor
versions may contain breaking changes).

## [0.3.0] - Unreleased

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

[0.3.0]: https://github.com/gh0stintheshe11/Rust-2D-Game-Engine/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/gh0stintheshe11/Rust-2D-Game-Engine/releases/tag/v0.2.0
