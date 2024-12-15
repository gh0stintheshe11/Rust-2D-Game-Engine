use crate::ecs::{AttributeType, AttributeValue};

pub struct EntityDefinition {
    pub name: &'static str, // Change to a static string slice
    pub attributes: &'static [(&'static str, AttributeType, AttributeValue)], // Use static slices
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
];
