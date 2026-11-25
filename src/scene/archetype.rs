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

    /// Whether the entity has every attribute of `preset`.
    pub fn has_preset(&self, preset: &AttributePreset) -> bool {
        preset
            .attributes
            .iter()
            .all(|(name, _)| self.attribute(name).is_some())
    }

    /// Add the attributes of `preset` the entity doesn't have yet, with the
    /// preset's values; existing attributes keep theirs. Returns how many
    /// were added.
    pub fn apply_preset(&mut self, preset: &AttributePreset) -> usize {
        preset
            .attributes
            .iter()
            .filter(|(name, value)| {
                self.attribute(name).is_none()
                    && self
                        .create_attribute(name, value.data_type(), value.clone())
                        .is_ok()
            })
            .count()
    }
}

/// A set of optional attributes that turns on an engine feature, such as a
/// trigger zone or a sprite sheet animation, with working default values.
/// The editor's inspector offers them, so the attribute names and types
/// don't have to be looked up.
#[derive(Clone, Debug, PartialEq)]
pub struct AttributePreset {
    /// The feature area: "Physics", "Sprite", "Animation" or "Effects".
    pub group: &'static str,
    pub name: &'static str,
    /// One line on what the attributes do.
    pub description: &'static str,
    pub attributes: Vec<(&'static str, AttributeValue)>,
}

/// Every attribute preset, grouped, in the order the editor lists them.
pub fn attribute_presets() -> Vec<AttributePreset> {
    use AttributeValue::{Boolean, Float, Integer, Vector2};
    let text = |value: &str| AttributeValue::String(value.to_string());
    let preset = |group, name, description, attributes| AttributePreset {
        group,
        name,
        description,
        attributes,
    };
    vec![
        preset(
            "Physics",
            "Physics body",
            "Moves, collides and falls with gravity",
            vec![
                ("is_movable", Boolean(true)),
                ("has_gravity", Boolean(true)),
                ("has_collision", Boolean(true)),
                ("can_rotate", Boolean(true)),
                ("friction", Float(0.5)),
                ("restitution", Float(0.0)),
                ("density", Float(1.0)),
            ],
        ),
        preset(
            "Physics",
            "Static body",
            "A fixed obstacle: walls, floors, platforms",
            vec![
                ("is_movable", Boolean(false)),
                ("has_collision", Boolean(true)),
            ],
        ),
        preset(
            "Physics",
            "Kinematic body",
            "Moved only by scripts (set_velocity); pushes bodies without being pushed",
            vec![
                ("is_kinematic", Boolean(true)),
                ("has_collision", Boolean(true)),
            ],
        ),
        preset(
            "Physics",
            "Trigger zone",
            "Reports overlaps to on_collision / on_collision_end without blocking",
            vec![
                ("has_collision", Boolean(true)),
                ("is_sensor", Boolean(true)),
            ],
        ),
        preset(
            "Physics",
            "Collision layers",
            "Its layer (1 to 32) and the layers it collides with (\"1,3\"; empty for all)",
            vec![("collision_layer", Integer(1)), ("collides_with", text(""))],
        ),
        preset(
            "Physics",
            "Collider size",
            "The body's size in world units, instead of the image's",
            vec![
                ("collider_width", Float(32.0)),
                ("collider_height", Float(32.0)),
            ],
        ),
        preset(
            "Physics",
            "Circle collider",
            "A round body that fits inside the body size",
            vec![("collider_shape", text("circle"))],
        ),
        preset(
            "Physics",
            "Gravity scale",
            "Multiplies the global gravity for this body",
            vec![("gravity_scale", Float(1.0))],
        ),
        preset(
            "Physics",
            "Gravity field",
            "Attracts bodies with has_gravity (strength / distance²)",
            vec![
                ("creates_gravity", Boolean(true)),
                (
                    "gravity_strength",
                    Float(crate::physics::DEFAULT_GRAVITY_FIELD_STRENGTH),
                ),
            ],
        ),
        preset(
            "Sprite",
            "Scale",
            "Multiplies the sprite's size (and its body's)",
            vec![("scale", Vector2(1.0, 1.0))],
        ),
        preset(
            "Sprite",
            "Rotation",
            "Turns the sprite around its center, in degrees",
            vec![("rotation", Float(0.0))],
        ),
        preset(
            "Sprite",
            "Mirror",
            "Flips the image horizontally (flip_x) or vertically (flip_y)",
            vec![("flip_x", Boolean(false)), ("flip_y", Boolean(false))],
        ),
        preset(
            "Sprite",
            "Opacity",
            "1 is opaque, 0 invisible",
            vec![("opacity", Float(1.0))],
        ),
        preset(
            "Sprite",
            "Tint",
            "Multiplies the image's colors: \"#rrggbb\" or \"#rrggbbaa\"",
            vec![("tint", text("#ffffff"))],
        ),
        preset(
            "Animation",
            "Sprite sheet",
            "Shows one frame of a grid of frame_size cells",
            vec![("frame_size", Vector2(32.0, 32.0)), ("frame", Integer(0))],
        ),
        preset(
            "Animation",
            "Animation",
            "Plays sprite sheet frames (\"0-3\") while the game runs",
            vec![
                ("frame_size", Vector2(32.0, 32.0)),
                ("frame", Integer(0)),
                ("animation", text("0-3")),
                ("animation_fps", Float(10.0)),
                ("animation_loop", Boolean(true)),
            ],
        ),
        preset(
            "Effects",
            "Particles",
            "Gives off particles (sparks, smoke, dust, rain); its image, if any, is their texture",
            crate::render::ParticleSettings::default().attributes(),
        ),
    ]
}
