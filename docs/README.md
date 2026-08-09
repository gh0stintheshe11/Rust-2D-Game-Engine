# Engine Docs

Per-module documentation for the current state of the code.

| Doc | Covers |
|---|---|
| [ecs.md](ecs.md) | Scene/Entity/Attribute data model (`src/ecs/`) |
| [physics_engine.md](physics_engine.md) | rapier2d wrapper: bodies, colliders, step/write-back (`src/physics_engine/`) |
| [render_engine.md](render_engine.md) | egui-painter renderer: camera, texture caches, render queue (`src/render_engine/`) |
| [lua_scripting.md](lua_scripting.md) | Lua session model, script environments, engine bindings (`src/lua_scripting/`) |
| [game_runtime.md](game_runtime.md) | Play-mode state machine and per-frame loop (`src/game_runtime/`) |
| [audio_engine.md](audio_engine.md) | rodio-based sound cache + playback (`src/audio_engine/`) |
| [input_handler.md](input_handler.md) | Per-frame egui input snapshot + context flag (`src/input_handler/`) |
| [project_manager.md](project_manager.md) | Project scaffolding, save/load, asset import, build (`src/project_manager/`) |
| [logger.md](logger.md) | Global in-memory console logger (`src/logger/`) |
| [editor_gui.md](editor_gui.md) | Editor shell, panels, viewport interaction, script editor (`src/engine_gui/` + `src/gui/`) |
| [course_report.md](course_report.md) | Archived course final report (Dec 2024) — historical reference only |

Convention: any PR that changes a module's behavior updates its doc.
