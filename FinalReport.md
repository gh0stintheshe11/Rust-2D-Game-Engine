A README.md file that contains the final report, in the form of a Markdown document, of no more than 5000 words in total length23. If you wish to include images (such as screenshots) in the final report, make sure that it can be visible when the instructor visits your GitHub repository with a web browser. The final report should include the following logistical and technical aspects of the project, described clearly and concisely:

## Team Members

- [Lang Sun - 1003584971 - ](https://github.com/gh0stintheshe11)
- [Feiyang Fan - 1005146913 - feiyang.fan@mail.utoronto.ca](https://github.com/feiyangfan)
- [Jie Peng(Frank) Chen - 997532861 - ](https://github.com/frankjc2022)

## Motivation

> Motivation: What motivated your team to spend time on this project? An excellent project idea is satisfying and fun to work on, and fills a gap that may not be easily found in the Rust ecosystem.

In the world of indie game development, having access to a dedicated, cross-platform 2D game engine can be transformative. A tool that strikes the perfect balance between simplicity and powerful features is essential for empowering indie developers and small studios to bring their creative visions to life without being overwhelmed by unnecessary complexity.

Rust’s strong performance and memory safety make it ideal for building high-efficiency applications like game engines. However, despite the growing ecosystem, the Rust community currently lacks a game engine explicitly tailored for 2D game development. While general-purpose engines like Bevy and Amethyst offer impressive functionality, their dual focus on 2D and 3D game development introduces a layer of complexity and overhead that can feel unnecessary and daunting for developers focused exclusively on 2D games.

Our project is an opportunity to address this gap while immersing ourselves in a deeply satisfying and enjoyable development process. Since all three of the members in the team enjoy playing game and game design, building a 2D game engine allows us to combine our passion for game design, Rust, and systems programming. The challenge of creating something lightweight yet robust, simple yet feature-rich, sparks our creativity and pushes our technical expertise. Knowing we might be enabling indie creators to focus on their visions without being bogged down by unnecessary complexity also brings us joy.

This engine is designed to be a "dummy-can-use" tool—intuitive enough for beginners to dive into game development while still providing advanced capabilities for experienced developers. By focusing solely on 2D, we eliminate the bloat and confusion that often come with multi-purpose engines, ensuring that every feature and optimization serves the unique needs of 2D game creators.

In essence, this project isn't just about building a game engine; it's about creating a space in the Rust ecosystem for indie developers and small studios to innovate, experiment, and succeed in the world of 2D game development.

## Objective

> Objectives: What are the objectives of this project?

The objective of this Rust 2D Game Engine project is to build a versatile, cross-platform engine that delivers the essential tools and core functionalities required for 2D game development. Designed for simplicity and efficiency, the engine will be lightweight, modular, and built with an indie developer-friendly focus, making it easy to integrate into a wide range of projects. 

## Features

> Features: What are the main features offered by the final project deliverable?

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

## User's Guide

> User’s (or Developer’s) Guide: How does a user — or developer, if the project is a crate — use each of the main features in the project deliverable?

## Reproducibility Guide

> Reproducibility Guide: What are the commands needed to set up the runtime environment, if any, and to build the project, so that its features can be used by a user or a developer? Note: The instructor will follow the steps you have included in this section, step-by-step, with no deviation. The instructor has access to a Ubuntu Linux server and a macOS Sonoma laptop computer.

## Contributions

> Contributions by each team member: What were the individual contributions by each member in the team?

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

## Lessons Learned

> Lessons learned and concluding remarks: Write about any lessons the team has learned throughout the project and concluding remarks, if any.
