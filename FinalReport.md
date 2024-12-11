## Table of Content



## Team Members

> #TODO: write preferred emails

-   [Lang Sun - 1003584971 - ](https://github.com/gh0stintheshe11)
-   [Feiyang Fan - 1005146913 - feiyang.fan@mail.utoronto.ca](https://github.com/feiyangfan)
-   [Jie Peng(Frank) Chen - 997532861 - ](https://github.com/frankjc2022)

## Motivation

As a team of passionate gamers and designers, we understand the transformative power of a dedicated game engine. All three of the members in the team enjoy playing game and game design, which is our motivation to create a tool that strikes the perfect balance between simplicity and powerful features, empowering indie developers and small studios to bring their creative visions to life without being overwhelmed by unnecessary complexity.

Rust's strong performance and memory safety make it ideal for building high-efficiency applications like game engines. However, despite the growing ecosystem, the Rust community currently lacks a game engine explicitly tailored for 2D game development. While general-purpose engines like Bevy and Amethyst offer impressive functionality, their dual focus on 2D and 3D game development introduces a layer of complexity and overhead that can feel unnecessary and daunting for developers focused exclusively on 2D games.

Our project is an opportunity to address this gap while immersing ourselves in a deeply satisfying and enjoyable development process. Building a 2D game engine allows us to combine our passion for game design, Rust, and systems programming. The challenge of creating something lightweight yet robust, simple yet feature-rich, sparks our creativity and pushes our technical expertise. Knowing we might be enabling indie creators to focus on their visions without being bogged down by unnecessary complexity also brings us joy.

This engine is designed to be a "dummy-can-use" tool—intuitive enough for beginners to dive into game development while still providing advanced capabilities for experienced developers. By focusing solely on 2D, we eliminate the bloat and confusion that often come with multi-purpose engines, ensuring that every feature and optimization serves the unique needs of 2D game creators.

In essence, this project isn't just about building a game engine; it's about creating a space in the Rust ecosystem for indie developers and small studios to innovate, experiment, and succeed in the world of 2D game development.

## Objective

The primary objective of our Rust 2D Game Engine project is to create a lightweight, cross-platform engine that empowers developers to build 2D games with simplicity and efficiency. By focusing on modularity, performance, and an indie developer-friendly approach, the engine aims to provide an accessible and robust foundation for game development.

Our project will emphasize:
#### Simplicity and Usability
Designing a user-friendly engine that lowers the barrier to entry for beginner game developers while supporting advanced use cases for experienced developers.

#### Performance and Scalability
Leveraging Rust's strengths in memory safety and high performance to ensure the engine is optimized for a wide range of 2D game projects, from small prototypes to larger, more complex games.

#### Modularity and Customization
Providing a flexible, modular architecture that allows developers to pick and integrate only the components they need, ensuring adaptability to different project requirements.

#### Cross-Platform Compatibility
Ensuring that games built with the engine can run seamlessly across multiple platforms, including desktop, web, and mobile environments.

#### Developer Empowerment
Streamlining the game development process by enabling rapid iteration and experimentation through intuitive tools, a Lua scripting system, and visual interfaces.

## Features

> Features: What are the main features offered by the final project deliverable?

#### Rendering Engine

_A cutting-edge graphics subsystem powered by [wgpu](https://crates.io/crates/wgpu), delivering cross-platform, hardware-accelerated rendering capabilities._

-   Leverages a modern, abstracted graphics API supporting multiple backends (Vulkan, Metal, DirectX 12, WebGPU) for platform flexibility.

-   Implements a high-performance rendering pipeline optimized for 2D sprite rendering, with texture management and dynamic shader compilation.

#### Physics Engine

_A robust 2D physics simulation system built on the [rapier2d](https://crates.io/crates/rapier2d) library, delivering realistic and responsive environmental interactions._

-   Integrates advanced physics calculations with support for collision detection, response, and physical simulations.

-   Offers rigid body dynamics, including dynamic and static body creation with customizable physical properties like mass, friction, and restitution.

-   Supports diverse collider geometries (spherical, cuboid, capsule) to accommodate varied game design requirements, from simple arcade-style to more intricate physics scenarios.

#### Entity Component System (ECS)

_An innovative architectural pattern designed for maximum flexibility, performance, and scalability in game entity management._

-   Implements a data-oriented design that separates game logic from data, enabling rapid feature development and minimal computational overhead.

-   Provides memory-efficient entity creation and management, capable of handling thousands of game objects with minimal performance degradation.

-   Offers seamless component composition, allowing developers to construct complex game behaviors through modular, reusable components.

#### Script Interpreter

_A powerful scripting integration layer enabling dynamic game logic extension and runtime behavior modification._

-   Seamlessly embeds Lua scripting via the [rlua](https://crates.io/crates/rlua) crate, allowing developers to write game logic, event handlers, and complex behaviors without recompiling the core engine.

-   Provides robust bidirectional data exchange between Rust and Lua, with error handling and safe memory management.

-   Supports hot-reloading of scripts, enabling rapid iteration and live modifications during game development.

#### Audio Engine

_A low-latency, high-fidelity audio processing system designed for immersive soundscapes and precise audio control._

-   Utilizes the [rodio](https://crates.io/crates/rodio) crate to deliver efficient audio stream management across multiple formats and playback scenarios.

-   Supports complex audio features like spatial sound, volume attenuation, and seamless music and sound effect transitions.

-   Ensures minimal audio latency, critical for maintaining synchronization with game events and player interactions.

#### Input Handling

_A responsive, platform-agnostic input management system that ensures smooth and intuitive player interactions._

-   Built with the [winit](https://crates.io/crates/winit) crate to provide device support, handling inputs from keyboards, mice, touchscreens, and game controllers.

-   Implements advanced input mapping and processing with low latency, supporting complex input combinations and gestures.

-   Offers configurable input schemes and easy integration with the ECS for flexible control mechanisms.

#### Game Project File Management

_An asset and project management system designed for streamlined game development workflows._

-   Provides robust file handling capabilities for game assets, configuration files, and persistent game states.

-   Supports project lifecycle management: creation, loading, saving, and cross-platform project building.

-   Implements intelligent asset tracking and dependency resolution to simplify project maintenance.

#### Engine GUI

_An intuitive, real-time development interface powered by [egui](https://crates.io/crates/egui), transforming game development into a more interactive and efficient process._

-   Offers a context-aware inspector for real-time modification of game components, entities, and system parameters.

-   Enables live debugging, performance profiling, and immediate visual feedback without interrupting the development workflow.

-   Provides customizable views and layouts, allowing developers to tailor the interface to their specific project needs and preferences.

## User's Guide

> User’s (or Developer’s) Guide: How does a user — or developer, if the project is a crate — use each of the main features in the project deliverable?

## Reproducibility Guide

> Reproducibility Guide: What are the commands needed to set up the runtime environment, if any, and to build the project, so that its features can be used by a user or a developer? Note: The instructor will follow the steps you have included in this section, step-by-step, with no deviation. The instructor has access to a Ubuntu Linux server and a macOS Sonoma laptop computer.

## Contributions

**Lang Sun**:

-   [Rendering Engine](#rendering-engine)
-   [Physics Engine](#physics-engine)
-   [Input Handling](#input-handling)

**Feiyang Fan**:

-   [Entity Component System (ECS)](#ecs)
-   [Audio Engine](#audio-engine)

**Frank Chen**:

-   [Script Interpreter](#script-interpreter)
-   [Game Project File Management](#game-project-file-management)
-   [Engine GUI](#engine-gui)

## Lessons Learned

> Lessons learned and concluding remarks: Write about any lessons the team has learned throughout the project and concluding remarks, if any.



