# Engine Docs

Per-module documentation for the current state of the code.

| Doc | Covers |
|---|---|
| [scene.md](scene.md) | Scene/Entity/Attribute data model (`src/scene/`) |
| [physics.md](physics.md) | rapier2d wrapper: bodies, colliders, step/write-back (`src/physics/`) |
| [render.md](render.md) | egui-painter renderer: camera, texture caches, render queue (`src/render/`) |
| [scripting.md](scripting.md) | Lua session model, script environments, engine bindings (`src/scripting/`) |
| [runtime.md](runtime.md) | Play-mode state machine and per-frame loop (`src/runtime/`) |
| [audio.md](audio.md) | rodio-based sound cache + playback (`src/audio/`) |
| [input.md](input.md) | Per-frame egui input snapshot + context flag (`src/input/`) |
| [project.md](project.md) | Project files and scene save/load (`src/project/`); scaffolding, asset import and build (`editor/src/project/`) |
| [logger.md](logger.md) | Global in-memory console logger (`src/logger/`) |
| [editor.md](editor.md) | Editor shell, panels, viewport interaction, script editor (`editor/src/`) |
| [course_report.md](course_report.md) | Archived course final report (Dec 2024) — historical reference only |

Convention: any PR that changes a module's behavior updates its doc.
