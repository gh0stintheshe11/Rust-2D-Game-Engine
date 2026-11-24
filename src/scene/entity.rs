use super::{Attribute, AttributeType, AttributeValue};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PhysicsProperties {
    pub is_movable: bool,
    pub affected_by_gravity: bool,
    pub creates_gravity: bool,
    pub has_collision: bool,
    pub friction: f32,
    pub restitution: f32,
    pub density: f32,
    pub can_rotate: bool,
}

impl Default for PhysicsProperties {
    fn default() -> Self {
        Self {
            is_movable: false,
            affected_by_gravity: false,
            creates_gravity: false,
            has_collision: true,
            friction: 0.5,
            restitution: 0.0,
            density: 1.0,
            can_rotate: false,
        }
    }
}

// =============== Entity (Manages Attributes) ===============
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entity {
    pub id: Uuid,
    pub name: String,
    pub attributes: IndexMap<Uuid, Attribute>,
    // Resource paths
    pub images: Vec<PathBuf>,    // Multiple images (sprites, textures)
    pub sounds: Vec<PathBuf>,    // Multiple sounds (effects, music)
    pub script: Option<PathBuf>, // Single script per entity
}

impl Entity {
    // Base entity creation - all entities get these attributes
    pub fn new(id: Uuid, name: &str) -> Result<Self, String> {
        let mut entity = Self {
            id,
            name: name.to_string(),
            attributes: IndexMap::new(),
            images: Vec::new(),
            sounds: Vec::new(),
            script: None,
        };

        // Core position attributes that cannot be deleted
        entity.create_attribute("x", AttributeType::Float, AttributeValue::Float(0.0))?;
        entity.create_attribute("y", AttributeType::Float, AttributeValue::Float(0.0))?;
        entity.create_attribute("z", AttributeType::Float, AttributeValue::Float(0.0))?;

        Ok(entity)
    }

    pub fn change_entity_name(&mut self, new_name: &str) -> Result<(), String> {
        if new_name.is_empty() {
            return Err("Entity name cannot be empty".to_string());
        }
        self.name = new_name.to_string();
        Ok(())
    }

    // Resource management methods
    pub fn add_image(&mut self, path: PathBuf) -> Result<(), String> {
        if !self.images.contains(&path) {
            self.images.push(path);
            Ok(())
        } else {
            Err("Image already exists".to_string())
        }
    }

    pub fn remove_image(&mut self, path: &PathBuf) -> Result<(), String> {
        if !self.images.contains(path) {
            return Err("Image not found".to_string());
        }
        self.images.retain(|p| p != path);
        Ok(())
    }

    pub fn add_sound(&mut self, path: PathBuf) -> Result<(), String> {
        if !self.sounds.contains(&path) {
            self.sounds.push(path);
            Ok(())
        } else {
            Err("Sound already exists".to_string())
        }
    }

    pub fn remove_sound(&mut self, path: &PathBuf) -> Result<(), String> {
        if !self.sounds.contains(path) {
            return Err("Sound not found".to_string());
        }
        self.sounds.retain(|p| p != path);
        Ok(())
    }

    pub fn set_script(&mut self, path: PathBuf) -> Result<(), String> {
        if self.script.is_some() {
            Err("Script already exists".to_string())
        } else {
            self.script = Some(path);
            Ok(())
        }
    }

    pub fn remove_script(&mut self) -> Result<(), String> {
        if self.script.is_none() {
            return Err("No script to remove".to_string());
        }
        self.script = None;
        Ok(())
    }

    // Helper methods to check resource existence
    pub fn has_image(&self, path: &PathBuf) -> bool {
        self.images.contains(path)
    }

    pub fn has_sound(&self, path: &PathBuf) -> bool {
        self.sounds.contains(path)
    }

    pub fn has_script(&self) -> bool {
        self.script.is_some()
    }

    pub fn list_images(&self) -> &Vec<PathBuf> {
        &self.images
    }

    pub fn list_sounds(&self) -> &Vec<PathBuf> {
        &self.sounds
    }

    pub fn get_image(&self, index: usize) -> Result<&PathBuf, String> {
        self.images
            .get(index)
            .ok_or_else(|| format!("Image at index {} not found", index))
    }

    pub fn get_sound(&self, index: usize) -> Result<&PathBuf, String> {
        self.sounds
            .get(index)
            .ok_or_else(|| format!("Sound at index {} not found", index))
    }

    pub fn get_script(&self) -> Option<&PathBuf> {
        self.script.as_ref()
    }

    // Attribute management
    pub fn create_attribute(
        &mut self,
        name: &str,
        data_type: AttributeType,
        value: AttributeValue,
    ) -> Result<Uuid, String> {
        // Check for duplicate names
        if self.get_attribute_by_name(name).is_ok() {
            return Err(format!("Attribute '{}' already exists", name));
        }

        let id = Uuid::new_v4();
        let attribute = Attribute {
            id,
            name: name.to_string(),
            data_type,
            value,
        };
        self.attributes.insert(id, attribute);
        Ok(id)
    }

    // Modify delete_attribute to protect physics attributes too
    pub fn delete_attribute(&mut self, id: Uuid) -> Result<bool, String> {
        if let Some(attr) = self.attributes.get(&id) {
            // Core position attributes that can't be deleted
            match attr.name.as_str() {
                // Core position attributes
                "x" | "y" | "z" => {
                    return Err("Cannot delete core position attributes".to_string());
                }
                // Physics-specific attributes
                "is_movable" | "has_gravity" | "creates_gravity" | "has_collision" | "friction"
                | "restitution" | "density" | "can_rotate" => {
                    if self.name.contains("physical") {
                        return Err(
                            "Cannot delete physics attributes from physical entity".to_string()
                        );
                    }
                }
                // Camera-specific attributes - expanded list
                "width" | "height" | "zoom" | "rotation" | "is_camera"
                    if self.name.contains("camera") =>
                {
                    return Err("Cannot delete camera attributes from camera entity".to_string());
                }
                _ => {}
            }
            Ok(self.attributes.shift_remove(&id).is_some())
        } else {
            Ok(false)
        }
    }

    pub fn list_attribute(&self) -> Vec<(Uuid, &str)> {
        self.attributes
            .iter()
            .map(|(id, attr)| (*id, attr.name.as_str()))
            .collect()
    }

    pub fn get_attribute(&self, id: Uuid) -> Result<&Attribute, String> {
        self.attributes
            .get(&id)
            .ok_or_else(|| format!("Attribute {} not found", id))
    }

    pub fn get_attribute_mut(&mut self, id: Uuid) -> Result<&mut Attribute, String> {
        self.attributes
            .get_mut(&id)
            .ok_or_else(|| format!("Attribute {} not found", id))
    }

    pub fn modify_attribute(
        &mut self,
        id: Uuid,
        new_name: Option<String>,
        new_type: Option<AttributeType>,
        new_value: Option<AttributeValue>,
    ) -> Result<(), String> {
        if let Some(attr) = self.attributes.get_mut(&id) {
            if attr.name == "is_camera" {
                return Err("Cannot modify is_camera attribute".to_string());
            }

            if let Some(name) = new_name {
                attr.name = name;
            }
            if let Some(data_type) = new_type {
                attr.data_type = data_type;
            }
            if let Some(value) = new_value {
                attr.value = value;
            }
            Ok(())
        } else {
            Err("Attribute not found".to_string())
        }
    }

    pub fn get_attribute_by_name(&self, name: &str) -> Result<&Attribute, String> {
        self.attributes
            .iter()
            .find(|(_, attr)| attr.name == name)
            .map(|(_, attr)| attr)
            .ok_or_else(|| format!("Attribute '{}' not found", name))
    }

    // Predefined: Camera Entity
    pub fn new_camera(id: Uuid, name: &str) -> Result<Self, String> {
        let mut entity = Self::new(id, name)?; // Get base attributes first

        // Add camera-specific attributes
        entity.create_attribute("width", AttributeType::Float, AttributeValue::Float(800.0))?; // Default width
        entity.create_attribute("height", AttributeType::Float, AttributeValue::Float(600.0))?; // Default height
        entity.create_attribute("zoom", AttributeType::Float, AttributeValue::Float(1.0))?;
        entity.create_attribute("rotation", AttributeType::Float, AttributeValue::Float(0.0))?;
        entity.create_attribute(
            "is_camera",
            AttributeType::Boolean,
            AttributeValue::Boolean(true),
        )?;

        Ok(entity)
    }

    // Predefined: Physical Entity
    pub fn new_physical(
        id: Uuid,
        name: &str,
        position: (f32, f32, f32), // Now accepts z coordinate
        physics: PhysicsProperties,
    ) -> Result<Self, String> {
        let mut entity = Self::new(id, name)?; // Get base attributes first

        // Set initial position
        entity.set_position(position.0, position.1, position.2)?;

        // Add physics-specific attributes
        entity.create_attribute(
            "is_movable",
            AttributeType::Boolean,
            AttributeValue::Boolean(physics.is_movable),
        )?;
        entity.create_attribute(
            "has_gravity",
            AttributeType::Boolean,
            AttributeValue::Boolean(physics.affected_by_gravity),
        )?;
        entity.create_attribute(
            "creates_gravity",
            AttributeType::Boolean,
            AttributeValue::Boolean(physics.creates_gravity),
        )?;
        entity.create_attribute(
            "has_collision",
            AttributeType::Boolean,
            AttributeValue::Boolean(physics.has_collision),
        )?;
        entity.create_attribute(
            "friction",
            AttributeType::Float,
            AttributeValue::Float(physics.friction),
        )?;
        entity.create_attribute(
            "restitution",
            AttributeType::Float,
            AttributeValue::Float(physics.restitution),
        )?;
        entity.create_attribute(
            "density",
            AttributeType::Float,
            AttributeValue::Float(physics.density),
        )?;
        entity.create_attribute(
            "can_rotate",
            AttributeType::Boolean,
            AttributeValue::Boolean(physics.can_rotate),
        )?;

        Ok(entity)
    }

    // Helper methods for position
    pub fn set_x(&mut self, x: f32) -> Result<(), String> {
        if let Ok(attr) = self.get_attribute_by_name("x") {
            self.modify_attribute(attr.id, None, None, Some(AttributeValue::Float(x)))?;
            Ok(())
        } else {
            Err("X attribute not found".to_string())
        }
    }

    pub fn set_y(&mut self, y: f32) -> Result<(), String> {
        if let Ok(attr) = self.get_attribute_by_name("y") {
            self.modify_attribute(attr.id, None, None, Some(AttributeValue::Float(y)))?;
            Ok(())
        } else {
            Err("Y attribute not found".to_string())
        }
    }

    pub fn set_z(&mut self, z: f32) -> Result<(), String> {
        if let Ok(attr) = self.get_attribute_by_name("z") {
            self.modify_attribute(attr.id, None, None, Some(AttributeValue::Float(z)))?;
            Ok(())
        } else {
            Err("Z attribute not found".to_string())
        }
    }

    pub fn set_position(&mut self, x: f32, y: f32, z: f32) -> Result<(), String> {
        self.set_x(x)?;
        self.set_y(y)?;
        self.set_z(z)?;
        Ok(())
    }

    pub fn get_x(&self) -> f32 {
        self.get_attribute_by_name("x")
            .and_then(|attr| {
                if let AttributeValue::Float(x) = attr.value {
                    Ok(x)
                } else {
                    Err("Attribute value is not a float".to_string())
                }
            })
            .unwrap_or(0.0)
    }

    pub fn get_y(&self) -> f32 {
        self.get_attribute_by_name("y")
            .and_then(|attr| {
                if let AttributeValue::Float(y) = attr.value {
                    Ok(y)
                } else {
                    Err("Attribute value is not a float".to_string())
                }
            })
            .unwrap_or(0.0)
    }

    pub fn get_z(&self) -> f32 {
        self.get_attribute_by_name("z")
            .and_then(|attr| {
                if let AttributeValue::Float(z) = attr.value {
                    Ok(z)
                } else {
                    Err("Attribute value is not a float".to_string())
                }
            })
            .unwrap_or(0.0)
    }

    pub fn get_position(&self) -> Result<(f32, f32, f32), String> {
        Ok((self.get_x(), self.get_y(), self.get_z()))
    }

    // Camera attribute getters
    pub fn get_camera_width(&self) -> f32 {
        self.get_attribute_by_name("width")
            .and_then(|attr| {
                if let AttributeValue::Float(w) = attr.value {
                    Ok(w)
                } else {
                    Err("Attribute value is not a float".to_string())
                }
            })
            .unwrap_or(800.0)
    }

    pub fn get_camera_height(&self) -> f32 {
        self.get_attribute_by_name("height")
            .and_then(|attr| {
                if let AttributeValue::Float(h) = attr.value {
                    Ok(h)
                } else {
                    Err("Attribute value is not a float".to_string())
                }
            })
            .unwrap_or(600.0)
    }

    pub fn get_camera_zoom(&self) -> f32 {
        self.get_attribute_by_name("zoom")
            .and_then(|attr| {
                if let AttributeValue::Float(z) = attr.value {
                    Ok(z)
                } else {
                    Err("Attribute value is not a float".to_string())
                }
            })
            .unwrap_or(1.0)
    }

    pub fn get_camera_rotation(&self) -> f32 {
        self.get_attribute_by_name("rotation")
            .and_then(|attr| {
                if let AttributeValue::Float(r) = attr.value {
                    Ok(r)
                } else {
                    Err("Attribute value is not a float".to_string())
                }
            })
            .unwrap_or(0.0)
    }

    // Camera attribute setters
    pub fn set_camera_width(&mut self, width: f32) -> Result<(), String> {
        if let Ok(attr) = self.get_attribute_by_name("width") {
            self.modify_attribute(attr.id, None, None, Some(AttributeValue::Float(width)))?;
            Ok(())
        } else {
            Err("Width attribute not found".to_string())
        }
    }

    pub fn set_camera_height(&mut self, height: f32) -> Result<(), String> {
        if let Ok(attr) = self.get_attribute_by_name("height") {
            self.modify_attribute(attr.id, None, None, Some(AttributeValue::Float(height)))?;
            Ok(())
        } else {
            Err("Height attribute not found".to_string())
        }
    }

    pub fn set_camera_zoom(&mut self, zoom: f32) -> Result<(), String> {
        if let Ok(attr) = self.get_attribute_by_name("zoom") {
            self.modify_attribute(attr.id, None, None, Some(AttributeValue::Float(zoom)))?;
            Ok(())
        } else {
            Err("Zoom attribute not found".to_string())
        }
    }

    pub fn set_camera_rotation(&mut self, rotation: f32) -> Result<(), String> {
        if let Ok(attr) = self.get_attribute_by_name("rotation") {
            self.modify_attribute(attr.id, None, None, Some(AttributeValue::Float(rotation)))?;
            Ok(())
        } else {
            Err("Rotation attribute not found".to_string())
        }
    }

    pub fn set_camera_size(&mut self, width: f32, height: f32) -> Result<(), String> {
        self.set_camera_width(width)?;
        self.set_camera_height(height)?;
        Ok(())
    }

    pub fn is_camera(&self) -> bool {
        self.get_attribute_by_name("is_camera")
            .and_then(|attr| {
                if let AttributeValue::Boolean(is_cam) = attr.value {
                    Ok(is_cam)
                } else {
                    Err("Attribute value is not a boolean".to_string())
                }
            })
            .unwrap_or(false)
    }
}
