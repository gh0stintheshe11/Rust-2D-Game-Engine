//! Built-in entity templates.

use super::{AttributeType, AttributeValue, Entity};

/// Attributes of the "Physics" template, added on top of the base `x`/`y`/`z`.
///
/// Used by the editor's "Create → Physics" item and by the
/// `create_physical_entity` script binding.
pub const PHYSICS_ATTRIBUTES: &[(&str, AttributeType, AttributeValue)] = &[
    (
        "position",
        AttributeType::Vector2,
        AttributeValue::Vector2(0.0, 0.0),
    ),
    (
        "is_movable",
        AttributeType::Boolean,
        AttributeValue::Boolean(true),
    ),
    (
        "has_gravity",
        AttributeType::Boolean,
        AttributeValue::Boolean(true),
    ),
    (
        "creates_gravity",
        AttributeType::Boolean,
        AttributeValue::Boolean(false),
    ),
    (
        "has_collision",
        AttributeType::Boolean,
        AttributeValue::Boolean(true),
    ),
    ("friction", AttributeType::Float, AttributeValue::Float(0.5)),
    (
        "restitution",
        AttributeType::Float,
        AttributeValue::Float(0.0),
    ),
    ("density", AttributeType::Float, AttributeValue::Float(1.0)),
    (
        "can_rotate",
        AttributeType::Boolean,
        AttributeValue::Boolean(true),
    ),
];

impl Entity {
    /// Add every attribute of the "Physics" template the entity doesn't have
    /// yet. Existing attributes keep their current values.
    pub fn add_physics_attributes(&mut self) {
        for (name, data_type, value) in PHYSICS_ATTRIBUTES {
            // Only fails when the attribute already exists, which is fine
            let _ = self.create_attribute(name, data_type.clone(), value.clone());
        }
    }
}
