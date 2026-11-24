use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// =============== Attribute Types ===============
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Attribute {
    pub id: Uuid,
    pub name: String,
    pub data_type: AttributeType,
    pub value: AttributeValue,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AttributeType {
    Integer,
    Float,
    String,
    Boolean,
    Vector2,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AttributeValue {
    Integer(i32),
    Float(f32),
    String(String),
    Boolean(bool),
    Vector2(f32, f32),
}

impl fmt::Display for AttributeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttributeValue::Integer(value) => write!(f, "{}", value),
            AttributeValue::Float(value) => {
                // Show .0 if it doesnt have decimal
                if value.fract() == 0.0 {
                    write!(f, "{:.1}", value)
                } else {
                    write!(f, "{}", value)
                }
            }
            AttributeValue::String(value) => write!(f, "{}", value),
            AttributeValue::Boolean(value) => write!(f, "{}", value),
            AttributeValue::Vector2(x, y) => write!(f, "{}, {}", x, y),
        }
    }
}
