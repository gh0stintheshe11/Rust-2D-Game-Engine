# Rust 2D Game Engine

## Table of Contents

- [Entity Component System](#ecs-entity-component-system)
- [Render Engine](#render-engine)
- [Physics Engine](#physics-engine)
- [Input Handler](#input-handling)
- [Audio Engine](#audio-engine)
- [Script Interpreter](#script_interpreter)
- [Game Project File Management](#file_system)
- [Engine GUI](#gui)

## [Render Engine](/src/render_engine.rs)
Using [wgpu](https://github.com/gfx-rs/wgpu) for the game rendering engine.
- [x] initial implementation

## [Physics Engine](/src/physics_engine.rs)
Using [rapier2d](https://github.com/dimforge/rapier) for the game physics engine.
- [x] initial implementation

## [ECS Entity Component System](/src/ecs.rs)
Functions that can modify the atrribute of entity on the fly. 
To be implemented...

## [Script Interpreter](/src/script_interpreter.rs)
For the game logic language, ```lua``` is a simple and popular choice in the game industry.
Using [rlua](https://github.com/Kampfkarren/rlua) for the game script interpreter.
- [x] initial implementation

## [Audio Engine](/src/audio_engine.rs)
Using [rodio](https://github.com/RustAudio/rodio) for the game audio engine.
- [x] initial implementation

## [Input Handling](/src/input_handler.rs)
Using [winit](https://github.com/rust-windowing/winit) for the game input handling.
- [x] initial implementation

> [!NOTE]
> Will add more script languages in the future if have time, such as C# and python.

## [Game Project File Management](/src/project_manager.rs)
A game engine should be able to display and manage the game project files.

- [x] create a new project
- [x] open a project
- [ ] save a project
- [ ] build a project

> [!NOTE]
> Not sure if project files need to be saved manually for now, since the project is directly modified in the engine.

## [Engine GUI](/src/engine_gui.rs)
Using [egui](https://github.com/emilk/egui) for the engine GUI.
- [x] initial implementation
