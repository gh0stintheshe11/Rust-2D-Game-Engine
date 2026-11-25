# Projects (`src/project/`, `editor/src/project/`)

A **project** is a folder containing `project.epm`. The engine reads and
writes project files (`src/project/`); creating projects, importing assets
and building games are editor features (`editor/src/project/`).

## Layout

```
my_game/
├── project.epm              # metadata (JSON): project_name, version
├── scenes/scene_manager.json
├── assets/{images,sounds,scripts}/
├── Cargo.toml               # the game's cargo package
├── src/main.rs              # calls rust_2d_game_engine::player::run
└── .gitignore               # /target
```

Only `project.epm` is required: version control drops empty folders, so a
cloned project may lack some `assets/` folders, and the editor creates them on
import.

## Engine: `ProjectManager`

- `load_project_full(dir)` → `LoadedProject { metadata, scene_manager }`, and
  makes `dir` the current project. `load_project` / `save_project` handle
  `project.epm` alone; `save_project_full` saves both files.
- `save_scene_hierarchy` / `load_scene_hierarchy` handle
  `scenes/scene_manager.json`.
- `set_project_path` / `get_project_path`: the current project, which script
  bindings resolve asset paths against (`resolve_asset_path`).
- `is_valid_project_directory(dir)`: `project.epm` exists.

### Paths

Asset and script paths are stored **relative to the project root with `/`
separators**, and resolved to absolute paths for this machine when loading.
In memory, entities hold absolute paths. Scene files written by older versions
contain absolute paths, possibly from another OS; they are remapped by
locating their `assets/` folder (Windows paths are recognized on Unix and vice
versa).

`project.epm` does not store the project's location (`ProjectMetadata::
project_path` is filled in on load), so a project can be moved or cloned
freely.

`make_entity_paths_relative(entity, project)` and `resolve_entity_paths(entity,
project)` apply the same conversions to a single entity, e.g. for the editor's
entity clipboard: copied with project-relative paths, resolved against the
project it is pasted into.

### Saving

Files are written to a temporary file next to the target and then renamed
over it, so a crash mid-save can't leave a truncated project behind.

## Editor: `editor/src/project/`

- **`create_project(dir)`**: refuses existing non-empty folders, creates the
  layout above, a starter scene (active, with a camera), and the game's cargo
  package. The package name is derived from the folder name (prefixed with
  `game_` when it would start with a non-letter or clash with a reserved
  name). The engine dependency is the local engine checkout the editor was
  built from when it exists, otherwise the engine release matching the
  editor's version (`git = <repository>, tag = "v<version>"`), so editors
  downloaded from GitHub releases create buildable projects.
- **`import_asset(project, file, type)`**: checks the extension (images:
  png/jpg/jpeg/gif, sounds: wav/mp3/ogg/flac, scripts: lua), copies the file into
  `assets/<type>/`, refuses name collisions, and returns the project-relative
  path.
- **`build_project(project)`**: runs `cargo build --release --target-dir
  <project>/target` (cargo is also looked for in `~/.cargo/bin` when it isn't
  on the editor's PATH), streams the output to the console, and copies
  `assets/` and `scenes/` into `target/release/`, which is then a complete
  game folder. The engine's `LICENSE` and `NOTICE` are written to
  `target/release/licenses/rust-2d-game-engine/`: Apache-2.0 requires them to
  accompany the game. Licenses of the game's other dependencies are not
  collected (tools like `cargo-about` can do that).

## Known limitations

- One open project per process: the current project path is global.
- No format versioning or migrations for `project.epm` and scene files.
- Builds copy files into `target/release/` without removing stale ones.
- `import_asset` copies flat (no subfolders) and doesn't rename on
  collisions.
