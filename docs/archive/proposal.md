# ECE 1724 Project Proposal: Rust 2D Game Engine

## Team Members

- [Lang Sun - 1003584971](https://github.com/gh0stintheshe11)
- [Feiyang Fan - 1005146913](https://github.com/feiyangfan)
- [Jie Peng(Frank) Chen - 997532861](https://github.com/frankjc2022)

> [!NOTE]
> The order of team members is based on the chronological order of joining the team.

⭐ **Project GitHub Repository**: [Rust-2D-Game-Engine](https://github.com/gh0stintheshe11/Rust-2D-Game-Engine)

## Table of Contents
- [Team Members](#team-members)
- [Table of Contents](#table-of-contents)
- [Motivation](#motivation)
- [Objective and Key Features](#objective-and-key-features)
- [Project Plan](#project-plan)

## Motivation

In the world of indie game development, a dedicated, cross-platform 2D game engine can be transformative, especially one that balances simplicity with powerful features. A simple, easy-to-use, and cross-platform 2D game engine is essential for indie developers and small studios focused on 2D games. However, the current Rust ecosystem lacks a dedicated engine specifically designed for 2D game development. Rust’s strong performance and memory safety make it ideal for building high-efficiency applications like game engines.

While Rust has game engines like Bevy and Amethyst, there's a notable gap for a lightweight, dedicated 2D game engine that prioritizes simplicity and cross-platform support. Existing solutions often combine 2D and 3D capabilities, resulting in unnecessary complexity for developers focused solely on 2D game development. This project aims to fill this gap by creating a specialized (**dummy-can-use** 😉) 2D engine that leverages Rust's memory safety and performance while maintaining a focused, user-friendly approach that's particularly suitable for indie developers and small studios.

## Objective and Key Features

The objective of this Rust 2D Game Engine project is to build a versatile, cross-platform engine that delivers the essential tools and core functionalities required for 2D game development. Designed for simplicity and efficiency, the engine will be lightweight, modular, and built with an indie developer-friendly focus, making it easy to integrate into a wide range of projects. 
Below are the main features that define this project:

#### Rendering Engine:

_Core component responsible for all graphical output, utilizing [wgpu](https://crates.io/crates/wgpu) crate, a cross-platform, modern graphics API for efficient rendering._

- Features hardware-accelerated rendering, multiple graphics backend support (Vulkan, Metal, DX12, WebGPU), and 2D sprite rendering.

- Includes texture creation and management, shader compilation, and a high-performance rendering pipeline.

#### Physics Engine:

_Core component providing basic 2D collision detection, response, and realistic physical simulations, suitable for arcade-style physics._

- Utilizes [rapier2d](https://crates.io/crates/rapier2d) crate, a powerful Rust physics engine, for complex physics calculations and interactions.

- Features include gravity simulation, dynamic and static rigid body creation, collision handling, and custom physical properties (mass, friction, restitution).

- Supports various collider shapes such as ball, cuboid, and capsule.

#### Entity Component System (ECS):

_Implements a flexible Entity Component System (ECS) pattern, allowing efficient creation, management, and updating of game entities and their states._

- This structure makes it easy to add new game features and behaviors without significant restructuring, promoting modularity and scalability.

- The ECS also optimizes memory usage and processing efficiency, ensuring that even large numbers of entities can be managed seamlessly.

#### Script Interpreter:

_Embeds a scripting interpreter for custom game logic, enabling developers to extend behavior without modifying the core engine._

- Uses Lua, a popular choice in the game industry, via the [rlua](https://crates.io/crates/rlua) crate to seamlessly integrate scripting within Rust.

- Supports running Lua scripts, data exchange between Rust and Lua, and effective error handling.

#### Audio Engine:

_Enables low-latency playback of sound effects and background music using the [rodio](https://crates.io/crates/rodio) crate to manage audio streams and control sound output._

- Designed to support various audio formats and streaming capabilities, it caters to the needs of both casual and immersive game sound design.

#### Input Handling:

_A responsive input handling system that processes player inputs (keyboard, mouse, game controllers) with low latency, providing a smooth and intuitive player experience._

- Built with the [winit](https://crates.io/crates/winit) crate, this feature ensures compatibility with a range of devices across platforms, handling core input types like clicks, keystrokes, and joystick movements.

#### Game Project File Management:

_Provides a file management system for handling game assets, configuration files, and saved states._

- Supports essential project management functions: creating, opening, saving, and building projects.

#### Engine GUI:

_The engine’s GUI provides a user-friendly interface, built with [egui](https://crates.io/crates/egui), allowing developers to interact with and manage game properties in real time._

- Features include the ability to inspect, modify, and save changes to game components and settings without needing to write additional code, streamlining the development process.

- This graphical interface enhances accessibility, making it easier for developers to experiment with and fine-tune elements directly from the GUI, ultimately speeding up iteration cycles.

## Project Plan

To achieve the project objective within the course timeframe, this team will divide responsibilities based on feature requirements and each member's focus areas and integrate the features together. following is the development plan:

### Feature Development of Each Member:

**Lang Sun**:

- [Rendering Engine](#rendering-engine)
- [Physics Engine](#physics-engine)
- [Input Handling](#input-handling)

**Feiyang Fan**: 

- [Entity Component System (ECS)](#ecs)
- [Audio Engine](#audio-engine)

**Frank Chen**:

- [Script Interpreter](#script-interpreter)
- [Game Project File Management](#game-project-file-management)
- [Engine GUI](#engine-gui)

> [!WARNING]
> The feature development of each member is not strictly separated. Some members are more experienced in certain features or game/game engine development and features will also be adjusted by all members later in feature integration. Therefore, members will contribute to the project as they see fit. The final contribution of each member may vary.

### Project Timeline

| date     | Week | Task |
| ---      | ---  | ---  |
| Previous | 0    | Project framework setup |
| 11/01 - 11/07    | 1    | feature development |
| 11/08 - 11/14    | 2    | feature development |
| 11/15 - 11/21    | 3    | adjustment for integration and actual integration |
| 11/22 - 11/28    | 4    | integration |
| 11/29 - 12/05    | 5    | testing and bug fixing |
| 12/06 - 12/15    | 6    | buffer for documentation and miscellaneous |
| 12/16 | 7    | Final Deliverable |

> [!IMPORTANT]
> This project is started very early of the course (way before the project proposal), so the framework setup already covers many of the features and related functionalities of the engine. The rest of the development can be well managed and strictly followed the plan as proposed.