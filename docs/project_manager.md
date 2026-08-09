# Project Manager (`src/project_manager/`)

Stateless utility (all associated functions on a unit struct, plus one global `RwLock<Option<String>>` holding the "current project path") for managing game projects on disk: scaffolding a new project, loading/saving metadata (`project.epm`) and the scene hierarchy (`scenes/scene_manager.json`), importing assets into typed folders, and driving `cargo build --release` for the game.

## Key types

| Type | Responsibility |
|---|---|
| `ProjectManager` | Unit struct; all functionality is associated functions |
| `ProjectMetadata` | `project_name`, `version`, `project_path` (absolute), `default_scene`, `active_scene_id` — serialized as JSON into `project.epm` |
| `LoadedProject` | Bundle of `ProjectMetadata` + deserialized `SceneManager` |
| `AssetType` | `Image` (png/jpg/jpeg/gif), `Sound` (wav/mp3/ogg), `Font` (ttf/otf), `Script` (lua); `valid_extensions()` drives import validation |

## Generated project layout

```
project_root/
├── project.epm              # metadata JSON (the "is a project" marker)
├── Cargo.toml               # generated, see limitations
├── src/main.rs              # generated eframe game shell
├── assets/{images,sounds,fonts,scripts}/
└── scenes/scene_manager.json
```

## Interactions with other modules

- **Editor GUI menus**: `file_menu` calls `create_project`/`load_project_full`; `import_menu` calls `import_asset`; `project_menu` runs `build_project` on a background thread.
- **`ecs`**: the entire `SceneManager` is serialized/deserialized here.
- **`lua_scripting`**: calls `ProjectManager::get_project_path()` to resolve script-supplied relative asset paths to absolute ones.
- **`logger`**: build output is streamed line-by-line to `LOGGER.debug` (ANSI colors stripped) as well as stdout.
- **Generated `main.rs`**: the built game itself calls `load_scene_hierarchy` + `set_project_path` at startup, using the executable's directory as the project root.

## Public API overview

- **Global path**: `set_project_path`, `get_project_path`
- **Lifecycle**: `create_project` (scaffold + empty scene hierarchy), `load_project` (metadata only), `save_project`, `load_project_full`, `save_project_full`
- **Scenes**: `save_scene_hierarchy`, `load_scene_hierarchy` (rewrites entity resource paths on load, see below)
- **Assets**: `import_asset(project, file, type)` — validates extension, copies into `assets/<type>/`, rejects duplicates, returns the project-relative path
- **Build**: `build_project` — `cargo build --release` in the project dir, then copies `assets/` and `scenes/` into `target/release/`
- **Validation**: `is_valid_project_directory` (checks `project.epm` exists), `validate_project_structure` (checks required folders + scene file)

## Known limitations / TODO

- Generated projects reference the engine via the **absolute path of the engine checkout that built the editor** (baked in at compile time). Building the project on another machine requires editing the dependency (a commented git-dependency line is included in the generated `Cargo.toml`).
- **Absolute paths break portability.** `ProjectMetadata.project_path` is absolute, and entity resource paths end up absolute in `scene_manager.json` (Lua bindings join them with the project path; `load_scene_hierarchy` rewrites whatever it finds to absolute paths under the *current* project root). Moving a project relies entirely on that load-time rewrite.
- **The path rewrite is a substring hack.** It searches for `/assets/{type}` with forward slashes — Windows backslash paths and assets outside the recognized folders pass through untouched; font path rewriting is commented out.
- **`load_project` has a write side effect**: it rewrites `project_path` in the metadata and saves the file back to disk on every load. `save_scene_hierarchy` calls it too, so saving scenes also rewrites `project.epm`.
- **Global `PROJECT_PATH` allows exactly one open project per process** and creates hidden coupling (`lua_scripting` reads it at a distance).
- **`default_scene: "main.scene"` is dead** — no such file is ever created or read.
- **`version` is unused** — no migration/versioning of project or scene formats.
- `build_project` assumes `cargo` is on `PATH` and copies assets into `target/release/` without cleaning stale files; `import_asset` copies flat (no subfolders) and rejects name collisions rather than renaming.
