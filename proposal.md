# ECE 1724 Project Proposal: Rust 2D Game Engine

## Team Members

- [Lang Sun - 1003584971](https://github.com/gh0stintheshe11)
- [Feiyang Fan - 1005146913](https://github.com/feiyangfan)
- [Jie Peng(Frank) Chen - 997532861](https://github.com/frankjc2022)

**Project GitHub Repository**: [Rust-2D-Game-Engine](https://github.com/gh0stintheshe11/Rust-2D-Game-Engine)

## Table of Contents
- [Team Members](#team-members)
- [Table of Contents](#table-of-contents)
- [Motivation](#motivation)
- [Objective and Key Features](#objective-and-key-features)
- [Project Plan](#project-plan)

## Motivation

<!--
What motivated your team to spend time on this project? An excellent project idea is satisfying and fun to work on, and fills a gap that may not be easily found in the Rust ecosystem.

Motivation: 30% (out of 10 Points)

The motivation is sufficiently convincing, showing that the team has thought about the project thoroughly (10 Points)
The motivation is lackluster and not convincing. (6 Points)
The motivation is not mentioned in the proposal. (0 Point)
-->

In the world of indie game development, a dedicated, cross-platform 2D game engine can be transformative, especially one that balances simplicity with powerful features. A simple, easy-to-use, and cross-platform 2D game engine is essential for indie developers and small studios focused on 2D games. However, the current Rust ecosystem lacks a dedicated engine specifically designed for 2D game development. Rust’s strong performance and memory safety make it ideal for building high-efficiency applications like game engines. This project aims to fill this gap by leveraging Rust’s unique benefits to create a lightweight and modular engine tailored exclusively for 2D games, empowering developers to focus on creativity and productivity with cross-platform support.

## Objective and Key Features

<!--
What is the objective of this project? What are the key features to be built in the project to achieve this objective? In other words, what is the idea that the completed project is trying to implement? It would be excellent if the idea has some novelty, but it is also important that it is feasible to be implemented within the timeframe of this course. Novelty is represented by the fact that the project, while small in scale, may be filling a gap in the current Rust ecosystem.

Objective and key features: 30% (out of 10 Points)

The objective and key features of the proposal may be filling a gap in the current Rust ecosystem. (3 Bonus Points)
The objective and key features of the proposal are clearly defined, with a reasonable amount of work for each team member. (10 Points)
The objective and key features of the proposal, are not clearly defined; or the amount of work for each team member is not well defined or insufficient. (6 Points)
The objective of the proposal is not mentioned in the proposal. (0 Point)

-->

The objective of the Rust 2D Game Engine project is to build a versatile, cross-platform engine that delivers the essential tools and core functionalities required for 2D game development. Designed for simplicity and efficiency, the engine will be lightweight, modular, and built with an indie developer-friendly focus, making it easy to integrate into a wide range of projects. 
Below are the main features that define this project:

**Rendering Engine:**

- Core component responsible for all graphical output, utilizing [wgpu](https://crates.io/crates/wgpu) crate, a cross-platform, modern graphics API for efficient rendering.
<br>
- Features hardware-accelerated rendering, multiple graphics backend support (Vulkan, Metal, DX12, WebGPU), and 2D sprite rendering.
<br>
- Includes texture creation and management, shader compilation, and a high-performance rendering pipeline.

**Physics Engine:**

- Core component providing basic 2D collision detection, response, and realistic physical 
simulations, suitable for arcade-style physics.
<br>
- Utilizes [rapier2d](https://crates.io/crates/rapier2d) crate, a powerful Rust physics engine, for complex physics calculations and interactions.
<br>
- Features include gravity simulation, dynamic and static rigid body creation, collision handling, and custom physical properties (mass, friction, restitution).
- Supports various collider shapes such as ball, cuboid, and capsule.

**Entity Component System (ECS):**

- Implements a flexible Entity Component System (ECS) pattern, allowing efficient creation, management, and updating of game entities and their states.
<br>
- This structure makes it easy to add new game features and behaviors without significant restructuring, promoting modularity and scalability.
<br>
- The ECS also optimizes memory usage and processing efficiency, ensuring that even large numbers of entities can be managed seamlessly.

**Script Interpreter:**

- Embeds a scripting interpreter for custom game logic, enabling developers to extend behavior without modifying the core engine.
<br>
- Uses Lua, a popular choice in the game industry, via the [rlua](https://crates.io/crates/rlua) crate to seamlessly integrate scripting within Rust.
<br>
- Supports running Lua scripts, data exchange between Rust and Lua, and effective error handling.

**Audio Engine:**

- Enables low-latency playback of sound effects and background music using the [rodio](https://crates.io/crates/rodio) crate to manage audio streams and control sound output.
<br>
- Designed to support various audio formats and streaming capabilities, it caters to the needs of both casual and immersive game sound design.

**Input Handling:**

- A responsive input handling system that processes player inputs (keyboard, mouse, game controllers) with low latency, providing a smooth and intuitive player experience.
<br>
- Built with the winit crate, this feature ensures compatibility with a range of devices across platforms, handling core input types like clicks, keystrokes, and joystick movements.

**Game Project File Management:**

- Provides a file management system for handling game assets, configuration files, and saved states.
<br>
- Supports essential project management functions: creating, opening, saving, and building projects.

**Engine GUI:**

- The engine’s GUI provides a user-friendly interface, built with egui, allowing developers to interact with and manage game properties in real time.
<br>
- Features include the ability to inspect, modify, and save changes to game components and settings without needing to write additional code, streamlining the development process.
<br>
- This graphical interface enhances accessibility, making it easier for developers to experiment with and fine-tune elements directly from the GUI, ultimately speeding up iteration cycles.

## Project Plan

<!--
Briefly and concisely, describe how your team plans to achieve the project objective in a matter of weeks, with clear descriptions of responsibilities for each team member in the team. As the duration of the project is quite short, there is no need to include milestones and tentative dates.

Tentative plan: 40% (out of 10 Points)

The proposed plan is concise and clear, includes responsibilities for each team member, and a casual reader can be convinced that the project can be reasonably completed by the project due date. (10 Points)
The proposed plan has been included, but not clear to a casual reader. (6 Points)
The proposed plan is not comprehensible. (0 Point)
-->

To achieve the project objective within the course timeframe, this team will divide responsibilities based on feature requirements and each member's focus areas and integrate the features together. following is the development plan:

### Feature Development of Each Member:

**Lang Sun**:

- **Rendering Engine**: Develop the render engine struct using the [wgpu](https://crates.io/crates/wgpu) crate to handle 2D sprite rendering, texture management, shader compilation, and a high-performance rendering pipeline. 
<br>

- **Physics Engine**: Build the physics engine with basic 2D collision detection, gravity simulation, and collision handling using the  [rapier2d](https://crates.io/crates/rapier2d) crate, including dynamic and static rigid body creation with custom physical properties.
<br>

- **Input Handling**: Code the input handling system to support keyboard and mouse inputs across platforms, using the [winit](https://crates.io/crates/winit) crate.

**Feiyang Fan**: 

- **Entity Component System (ECS)**: Set up the ECS for flexible entity creation and efficient game-state management.
<br>

- **Audio Engine**: Develop the AudioEngine for low-latency playback of sound effects and background music, managing audio streams with the [rodio](https://crates.io/crates/rodio) crate.

**Frank Chen**:

- **Script Interpreter**: Integrate a Lua scripting interpreter to support custom game logic, utilizing the [rlua](https://crates.io/crates/rlua) crate for running Lua scripts, data exchange, and error handling.
<br>

- **Game Project File Management**: Implement a file management system for handling game assets, configuration files, and saved states, including functions to create, open, save, and build projects.
<br>

- **Engine GUI**: Implement a GUI for developers to inspect and edit game properties in real time, using the [egui](https://crates.io/crates/egui) crate.


### Project Timeline

| date     | Week | Task |
| ---      | ---  | ---  |
| Previous | 0    | Project framework setup |
| 11/01 - 11/07     | 1    | feature development |
| 11/08 - 11/14    | 2    | feature development |
| 11/15 - 11/21    | 3    | adjustment for integration and actual integration |
| 11/22 - 11/28    | 4    | integration |
| 11/29 - 12/05    | 5    | testing and bug fixing |
| 12/06 - 12/15    | 6    | buffer for documentation and miscellaneous |
| 12/16 | 7    | Final Deliverable |

