//! Rust 2D Game Engine: the runtime library linked into every game, and used
//! by the editor.
//!
//! | Module | Responsibility |
//! |---|---|
//! | [`scene`] | Data model: scenes, entities, typed attributes |
//! | [`physics`] | rapier2d world mirroring the scene's physics entities |
//! | [`render`] | Sprite rendering with egui, camera, texture caches |
//! | [`scripting`] | Lua 5.4 sessions and the engine bindings scripts call |
//! | [`audio`] | Sound playback |
//! | [`input`] | Per-frame keyboard/mouse snapshot |
//! | [`runtime`] | Play-mode loop tying the systems together |
//! | [`player`] | Standalone game window used by exported games |
//! | [`project`] | Project files: metadata and scene (de)serialization |
//! | [`logger`] | In-memory log shown in the editor console |

pub mod audio;
pub mod input;
pub mod logger;
pub mod physics;
pub mod player;
pub mod project;
pub mod render;
pub mod runtime;
pub mod scene;
pub mod scripting;

pub use eframe;
