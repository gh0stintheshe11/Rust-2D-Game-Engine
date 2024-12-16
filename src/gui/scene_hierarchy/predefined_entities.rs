use crate::ecs::{AttributeType, AttributeValue};

pub struct EntityDefinition {
    pub name: &'static str,
    pub attributes: &'static [(&'static str, AttributeType, AttributeValue)],
}

pub const PREDEFINED_ENTITIES: &[EntityDefinition] = &[
    EntityDefinition {
        name: "Camera",
        attributes: &[
            ("transform_position_x", AttributeType::Float, AttributeValue::Float(0.0)),
            ("transform_position_y", AttributeType::Float, AttributeValue::Float(0.0)),
            ("transform_position_z", AttributeType::Float, AttributeValue::Float(0.0)),
            ("transform_rotation_x", AttributeType::Float, AttributeValue::Float(0.0)),
            ("transform_rotation_y", AttributeType::Float, AttributeValue::Float(0.0)),
            ("transform_rotation_z", AttributeType::Float, AttributeValue::Float(0.0)),
            ("transform_scale_x", AttributeType::Float, AttributeValue::Float(1.0)),
            ("transform_scale_y", AttributeType::Float, AttributeValue::Float(1.0)),
            ("transform_scale_z", AttributeType::Float, AttributeValue::Float(1.0)),
        ],
    },
    EntityDefinition {
        name: "Physics",
        attributes: &[
            ("position", AttributeType::Vector2, AttributeValue::Vector2(0.0, 0.0)),
            ("is_movable", AttributeType::Boolean, AttributeValue::Boolean(true)),
            ("has_gravity", AttributeType::Boolean, AttributeValue::Boolean(true)),
            ("creates_gravity", AttributeType::Boolean, AttributeValue::Boolean(false)),
            ("has_collision", AttributeType::Boolean, AttributeValue::Boolean(true)),
            ("friction", AttributeType::Float, AttributeValue::Float(0.5)),
            ("restitution", AttributeType::Float, AttributeValue::Float(0.0)),
            ("density", AttributeType::Float, AttributeValue::Float(1.0)),
            ("can_rotate", AttributeType::Boolean, AttributeValue::Boolean(true)),
        ],
    },
];
