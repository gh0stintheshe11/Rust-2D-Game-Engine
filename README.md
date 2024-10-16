# Rust 2D Game Engine

## Table of Contents

- [Entity Component System](#ecs-entity-component-system)
- [Render Engine](#render-engine)
- [Physics Engine](#physics-engine)
- [Script Interpreter](#script_interpreter)
- [Game Project File Management](#file_system)
- [Engine GUI](#gui)

## Render Engine
Using [wgpu](https://github.com/gfx-rs/wgpu) as the game rendering engine.

## Physics Engine
Using [rapier2d](https://github.com/dimforge/rapier) as the game physics engine.

## ECS Entity Component System
To be implemented...

## Script Interpreter
For the game logic language, ```lua``` is a simple and popular choice in the game industry.
Using [rlua](https://github.com/Kampfkarren/rlua) as the game script interpreter.

> [!NOTE]
> Will add more script languages in the future if have time, such as C# and python.

## Game Project File Management
A game engine should be able to display and manage the game project files.

- [x] create a new project
- [x] open a project
- [ ] save a project
- [ ] build a project - to be implemented...

> [!NOTE]
> Not sure if project files need to be saved manually for now, since the project is directly modified in the engine.

## Engine GUI
Using [egui](https://github.com/emilk/egui) as the engine GUI.