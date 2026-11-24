use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use uuid::Uuid;

/// A named, typed value on an entity.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Attribute {
    pub id: Uuid,
    pub name: String,
    pub data_type: AttributeType,
    pub value: AttributeValue,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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
    Float(#[serde(deserialize_with = "finite_f32")] f32),
    String(String),
    Boolean(bool),
    Vector2(
        #[serde(deserialize_with = "finite_f32")] f32,
        #[serde(deserialize_with = "finite_f32")] f32,
    ),
}

impl AttributeValue {
    /// The type this value belongs to.
    pub fn data_type(&self) -> AttributeType {
        match self {
            AttributeValue::Integer(_) => AttributeType::Integer,
            AttributeValue::Float(_) => AttributeType::Float,
            AttributeValue::String(_) => AttributeType::String,
            AttributeValue::Boolean(_) => AttributeType::Boolean,
            AttributeValue::Vector2(_, _) => AttributeType::Vector2,
        }
    }

    /// Whether every number in the value is finite. Infinities and NaN
    /// can't be stored: JSON has no representation for them.
    pub fn is_finite(&self) -> bool {
        match self {
            AttributeValue::Float(v) => v.is_finite(),
            AttributeValue::Vector2(x, y) => x.is_finite() && y.is_finite(),
            _ => true,
        }
    }

    /// Check that the value can be stored in an attribute of `data_type`.
    pub fn validate_for(&self, data_type: &AttributeType) -> Result<(), String> {
        if &self.data_type() != data_type {
            return Err(format!(
                "a {:?} value can't be stored in a {:?} attribute",
                self.data_type(),
                data_type
            ));
        }
        if !self.is_finite() {
            return Err("numbers must be finite (not infinity or NaN)".to_string());
        }
        Ok(())
    }
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

/// Read a float that older versions may have saved as `null` (serde_json's
/// encoding of infinities and NaN): it becomes 0 instead of making the whole
/// scene file unreadable.
fn finite_f32<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f32, D::Error> {
    let value = Option::<f32>::deserialize(deserializer)?;
    Ok(value.filter(|v| v.is_finite()).unwrap_or_else(|| {
        crate::logger::LOGGER.warning(
            "A scene file contained an invalid number (infinity or NaN); it was replaced by 0",
        );
        0.0
    }))
}
