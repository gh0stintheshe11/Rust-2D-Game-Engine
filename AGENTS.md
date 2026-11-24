# AGENTS.md

Guidance for AI coding agents (and humans) working in this repository.

## Project

A 2D game engine with a built-in editor, written in Rust. The editor is an
egui/eframe app; game logic is Lua 5.4 (mlua); physics is rapier2d; audio is
rodio. The editor scaffolds game projects and builds them into standalone
executables that link the engine.

Cargo workspace:

| Path | Contents |
|---|---|
| `src/` | Engine library `rust-2d-game-engine` (crate `rust_2d_game_engine`): `scene`, `physics`, `render`, `scripting`, `audio`, `input`, `runtime`, `player`, `project`, `logger` |
| `tests/` | Engine integration tests; headless (no window or audio device needed) |
| `editor/` | Editor binary `rust-2d-game-editor`: app shell, panels, menus, project scaffolding/build |
| `demo/` | Example game project (a workspace member), also loaded by `tests/demo_integration_test.rs` |
| `docs/` | One design doc per module; `docs/README.md` is the index |

Exported games depend on the engine crate only; a game's `main.rs` is a
single call to `rust_2d_game_engine::player::run`.

## Commands

```bash
cargo run                                              # launch the editor (WSLg: WAYLAND_DISPLAY= cargo run)
cargo test --workspace                                 # full test suite
cargo clippy --workspace --all-targets -- -D warnings  # must be clean, CI enforces it
cargo fmt --all --check                                # must be clean, CI enforces it
```

Linux system packages needed to build are listed in `README.md`.

## Definition of done

1. `cargo test`, clippy and fmt (commands above) all pass.
2. Bug fixes come with a regression test whenever the behavior can be tested
   headlessly.
3. The module's doc in `docs/` describes the code as it now is, including its
   "Known limitations" section. Docs describe current behavior only — no
   history, no plans presented as facts.

## Code conventions

- No `unsafe`. Lua bindings share engine state through `Rc<RefCell<…>>`
  handles; never hold a `RefCell` borrow across a call into Lua, or a
  re-entrant binding will panic.
- Never panic on user data (project files, scenes, scripts, assets). Return an
  error and surface it in the editor console through `logger::LOGGER`.
- Scene files store asset paths project-relative with `/` separators; in memory
  they are absolute. Conversion happens only in `project`.
- The engine crate must not depend on the editor; editor-only functionality
  (scaffolding, asset import, builds, GUI) lives in `editor/`.
- Diagrams in docs are mermaid, laid out top-down (`graph TD`); rendering width
  is limited.

## Git

- Commit messages follow Conventional Commits: `type(scope): summary`, with
  types `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `build`, `ci`,
  `chore`. Imperative mood, subject ≤ 72 characters, body wrapped at 72
  explaining what changed and why. Describe only the delta.
- Group related small changes into one commit; keep unrelated changes in
  separate commits. Every commit must build and pass the checks above.
- Never push, force-push, tag, rebase, or rewrite history unless the maintainer
  explicitly asks.
