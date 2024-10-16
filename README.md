# Rust 2D Game Engine

## Table of Contents

- [Entity Component System](#ecs-entity-component-system)
- [Render Engine](#render-engine)
- [Physics Engine](#physics-engine)
- [Script Interpreter](#script_interpreter)
- [Game Project File Management](#file_system)
- [Engine GUI](#gui)

## [Render Engine](#render_engine)
Using [wgpu](https://github.com/gfx-rs/wgpu) for the game rendering engine.
- [x] initial implementation

## [Physics Engine](#physics_engine)
Using [rapier2d](https://github.com/dimforge/rapier) for the game physics engine.
- [x] initial implementation

## [ECS Entity Component System](#ecs_entity_component_system)
To be implemented...

## [Script Interpreter](#script_interpreter)
For the game logic language, ```lua``` is a simple and popular choice in the game industry.
Using [rlua](https://github.com/Kampfkarren/rlua) for the game script interpreter.
- [x] initial implementation

> [!NOTE]
> Will add more script languages in the future if have time, such as C# and python.

## [Game Project File Management](#game_project_file_management)
A game engine should be able to display and manage the game project files.

- [x] create a new project
- [x] open a project
- [ ] save a project
- [ ] build a project

> [!NOTE]
> Not sure if project files need to be saved manually for now, since the project is directly modified in the engine.

## [Engine GUI](#engine_gui)
Using [egui](https://github.com/emilk/egui) for the engine GUI.
- [x] initial implementation
