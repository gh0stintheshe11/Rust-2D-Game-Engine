//! The engine's data model.
//!
//! ```text
//! SceneManager
//! └── Manages multiple Scenes (+ cross-scene shared entities)
//!      Scene
//!      └── Manages Entities directly
//!          Entity
//!          └── Manages its own Attributes + resource paths
//! ```
//!
//! Split across submodules purely for readability; everything is re-exported
//! here so external code keeps using `crate::ecs::*` paths.

mod attribute;
mod entity;
mod scene;
mod scene_manager;

pub use attribute::{Attribute, AttributeType, AttributeValue};
pub use entity::{Entity, PhysicsProperties};
pub use scene::Scene;
pub use scene_manager::SceneManager;
