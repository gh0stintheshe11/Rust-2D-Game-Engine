use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::fs::remove_file;

/// Enum to represent different types of attribute values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttributeValueType {
    Integer(i32),
    Float(f32),
    String(String),
    Boolean(bool),
}

/// Struct to represent an attribute with a name and value type
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub value_type: AttributeValueType,
}

/// Struct to represent an entity that contains an ID and attributes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entity {
    pub id: usize,
    pub attributes: HashMap<String, Attribute>, // Stores attributes with name and value type
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
    pub entities: HashMap<usize, Entity>, // Map of entity IDs to entities
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

    /// Create a new entity, save it as a JSON file, and return it
    pub fn create_entity(&mut self) -> Entity {
        let entity = Entity::new(self.next_id);
        self.entities.insert(self.next_id, entity.clone());
        self.next_id += 1;
        entity
    }

    // Find the next available JSON file number in the directory
    fn find_next_available_json_number(&self, save_path: &str) -> Result<usize, String> {
        // Read the directory contents
        match fs::read_dir(save_path) {
            Ok(entries) => {
                let mut used_numbers = std::collections::HashSet::new();

                // Collect existing JSON file numbers
                for entry in entries {
                    if let Ok(entry) = entry {
                        if let Some(filename) = entry.file_name().to_str() {
                            if filename.ends_with(".json") && filename != "entity_index.json" {
                                if let Ok(number) =
                                    filename.trim_end_matches(".json").parse::<usize>()
                                {
                                    used_numbers.insert(number);
                                }
                            }
                        }
                    }
                }

                // Find the first unused number
                let mut next_number = 0;
                while used_numbers.contains(&next_number) {
                    next_number += 1;
                }

                Ok(next_number)
            }
            Err(e) => Err(format!("Failed to read directory {}: {}", save_path, e)),
        }
    }

    // Modify create_entity_path to use the next available JSON number
    pub fn create_entity_path(
        &mut self,
        save_path: &str,
        entity_name: &str,
    ) -> Result<Entity, String> {
        // Find the next available JSON file number
        let file_number = self.find_next_available_json_number(save_path)?;

        let mut entity = Entity::new(self.next_id);

        // Add name attribute
        match self.add_attribute_with_validation(
            &mut entity,
            "name".to_string(),
            AttributeValueType::String(String::new()),
            entity_name.to_string(),
        ) {
            Ok(_) => {
                // Insert the entity into the entities map using the file number
                self.entities.insert(file_number, entity.clone());

                // Attempt to save the entity as a JSON file
                match self.save_entity_as_json(file_number, save_path) {
                    Ok(_) => Ok(entity),
                    Err(err) => Err(format!("Failed to save entity: {}", err)),
                }
            }
            Err(err) => Err(format!("Failed to add name attribute: {}", err)),
        }
    }

    // /// Delete an entity
    // pub fn delete_entity(&mut self, entity: Entity) {
    //     self.entities.remove(&entity.id);
    // }

    /// Delete an entity by its number (ID) and remove the associated file
pub fn delete_entity_by_number(&mut self, entity_number: usize, root_dir: &str) -> Result<(), String> {
    // Define the full path to the JSON file for the entity
    let full_path = format!("{}/{}.json", root_dir, entity_number);

    // Check if the file exists
    if fs::metadata(&full_path).is_ok() {
        // Try to delete the file
        match remove_file(full_path) {
            Ok(_) => {
                // Optionally remove the entity from memory (if you want to)
                self.entities.remove(&entity_number);

                Ok(())
            }
            Err(e) => Err(format!(
                "Failed to delete the file for entity {}. Error: {}",
                entity_number, e
            )),
        }
    } else {
        Err(format!("Entity file {} does not exist.", full_path))
    }
}

    // /// Check if an entity exists
    // pub fn entity_exists(&self, entity: Entity) -> bool {
    //     self.entities.contains_key(&entity.id)
    // }

    // // Copy an existing entity and return the new one
    // pub fn copy_entity(&mut self, existing_entity: &Entity) -> Entity {
    //     let new_entity = Entity {
    //         id: self.next_id,
    //         attributes: existing_entity.attributes.clone(), // Copy all attributes
    //     };
    //     self.entities.insert(self.next_id, new_entity.clone());
    //     self.next_id += 1;
    //     new_entity
    // }

    // Save an entity as a JSON file in the specified path
    pub fn save_entity_as_json(&self, file_number: usize, root_dir: &str) -> Result<(), String> {
        if let Some(entity) = self.entities.get(&file_number) {
            // Define the full path dynamically using the root_dir
            let full_path = format!("{}/{}.json", root_dir, file_number);
    
            // Ensure the directory exists
            if let Err(e) = fs::create_dir_all(&root_dir) {
                return Err(format!(
                    "Failed to create directory '{}'. Error: {}",
                    root_dir, e
                ));
            }
    
            // Serialize the entity to JSON
            match serde_json::to_string_pretty(entity) {
                Ok(json_string) => {
                    // Write the JSON to a file
                    match File::create(&full_path) {
                        Ok(mut file) => {
                            if let Err(e) = file.write_all(json_string.as_bytes()) {
                                Err(format!(
                                    "Failed to write entity file: {}. Error: {}",
                                    full_path, e
                                ))
                            } else {
                                Ok(())
                            }
                        }
                        Err(e) => Err(format!(
                            "Failed to create entity file: {}. Error: {}",
                            full_path, e
                        )),
                    }
                }
                Err(e) => Err(format!("Failed to serialize entity to JSON. Error: {}", e)),
            }
        } else {
            Err(format!("Entity with ID {} does not exist.", file_number))
        }
    }
}
