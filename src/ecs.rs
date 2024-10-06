use std::collections::HashMap;

/// Enum to represent different types of attribute values
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValueType {
    Integer(i32),
    Float(f32),
    String(String),
    Boolean(bool),
}

/// Struct to represent an attribute with a name and value type
#[derive(Clone, Debug)]
pub struct Attribute {
    pub name: String,
    pub value_type: AttributeValueType,
}

/// Struct to represent an entity that contains an ID and attributes
#[derive(Clone, Debug)]
pub struct Entity {
    pub id: usize,
    pub attributes: HashMap<String, Attribute>,  // Stores attributes with name and value type
}

impl Entity {
    // Constructor for Entity
    pub fn new(id: usize) -> Self {
        Entity {
            id,
            attributes: HashMap::new(),
        }
    }
}

/// Manages all entities and provides functions for creating, deleting, and modifying entities
pub struct EntityManager {
    pub next_id: usize,
    pub entities: HashMap<usize, Entity>,  // Map of entity IDs to entities
}

impl EntityManager {
    /// Constructor for EntityManager
    pub fn new() -> Self {
        EntityManager {
            next_id: 0,
            entities: HashMap::new(),
        }
    }

    /// Validate the value based on the expected type
    fn validate_value(
        value_type: AttributeValueType,
        value: String,
    ) -> Result<AttributeValueType, String> {
        match value_type {
            AttributeValueType::Integer(_) => {
                if let Ok(parsed_value) = value.parse::<i32>() {
                    Ok(AttributeValueType::Integer(parsed_value))
                } else {
                    Err(format!("Invalid integer value: {}", value))
                }
            }
            AttributeValueType::Float(_) => {
                if let Ok(parsed_value) = value.parse::<f32>() {
                    Ok(AttributeValueType::Float(parsed_value))
                } else {
                    Err(format!("Invalid float value: {}", value))
                }
            }
            AttributeValueType::String(_) => Ok(AttributeValueType::String(value)),
            AttributeValueType::Boolean(_) => {
                if let Ok(parsed_value) = value.parse::<bool>() {
                    Ok(AttributeValueType::Boolean(parsed_value))
                } else {
                    Err(format!("Invalid boolean value: {}", value))
                }
            }
        }
    }

    /// Add an attribute to an entity with validation
    pub fn add_attribute_with_validation(
        &mut self,
        entity: &mut Entity,
        name: String,
        value_type: AttributeValueType,
        value: String,
    ) -> Result<(), String> {
        match Self::validate_value(value_type, value) {
            Ok(valid_value) => {
                let attribute = Attribute {
                    name: name.clone(),
                    value_type: valid_value,
                };
                entity.attributes.insert(name, attribute);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Modify an attribute value with validation
    pub fn modify_attribute_with_validation(
        &mut self,
        entity: &mut Entity,
        name: String,
        value_type: AttributeValueType,
        value: String,
    ) -> Result<(), String> {
        if entity.attributes.contains_key(&name) {
            match Self::validate_value(value_type, value) {
                Ok(valid_value) => {
                    if let Some(attribute) = entity.attributes.get_mut(&name) {
                        attribute.value_type = valid_value;
                    }
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            Err(format!("Attribute {} does not exist.", name))
        }
    }

    /// Delete an attribute from an entity
    pub fn delete_attribute(&mut self, entity: &mut Entity, name: &String) {
        entity.attributes.remove(name);
    }

    /// Check if an attribute exists for an entity
    pub fn attribute_exists(&self, entity: &Entity, name: &String) -> bool {
        entity.attributes.contains_key(name)
    }

    /// Create a new entity and return it
    pub fn create_entity(&mut self) -> Entity {
        let entity = Entity::new(self.next_id);
        self.entities.insert(self.next_id, entity.clone());
        self.next_id += 1;
        entity
    }

    /// Delete an entity
    pub fn delete_entity(&mut self, entity: Entity) {
        self.entities.remove(&entity.id);
    }

    /// Check if an entity exists
    pub fn entity_exists(&self, entity: Entity) -> bool {
        self.entities.contains_key(&entity.id)
    }

    // Copy an existing entity and return the new one
    pub fn copy_entity(&mut self, existing_entity: &Entity) -> Entity {
        let new_entity = Entity {
            id: self.next_id,
            attributes: existing_entity.attributes.clone(),  // Copy all attributes
        };
        self.entities.insert(self.next_id, new_entity.clone());
        self.next_id += 1;
        new_entity
    }
}