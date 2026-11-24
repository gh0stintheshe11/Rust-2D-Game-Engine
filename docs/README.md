# Documentation

One document per module, describing how the code works today (including its
known limitations). Any change to a module's behavior updates its document.

## Engine (`src/`)

| Document | Covers |
|---|---|
| [scene.md](scene.md) | Data model: scenes, entities, typed attributes, templates (`src/scene/`) |
| [physics.md](physics.md) | rapier2d bodies and colliders from attributes, forces, trigger zones, collision layers, raycasts, gravity fields (`src/physics/`) |
| [render.md](render.md) | Sprites (sheets and animation, mirroring, tint), text, camera framing, texture caches (`src/render/`) |
| [scripting.md](scripting.md) | Lua sessions, hooks, hot reload, sandbox, scenes and prefabs, the complete script API (`src/scripting/`) |
| [runtime.md](runtime.md) | Play-mode state machine, the per-frame loop, switching scenes (`src/runtime/`, `src/player.rs`) |
| [audio.md](audio.md) | Sound playback (`src/audio/`) |
| [input.md](input.md) | Keyboard and mouse state, key names (`src/input/`) |
| [project.md](project.md) | Project files and folders (`src/project/`); creating, importing, building (`editor/src/project/`) |
| [logger.md](logger.md) | The log behind the editor console (`src/logger/`) |

## Editor (`editor/`)

| Document | Covers |
|---|---|
| [editor.md](editor.md) | Panels, how they cooperate, saving and undo, viewport, script editor |

## Archive

Documents from the 2024 university course the project started as. They
describe the project at that time, not the current code.

- [Course project proposal](archive/proposal.md)
- [Course final report](archive/course_report.md)
