use uuid::Uuid;
use serde::{Serialize, Deserialize};
use std::fmt;
use indexmap::IndexMap;
use rayon::prelude::*;
use std::path::PathBuf;

//SceneManager
// └── Manages multiple Scenes
//      Scene
//      └── Manages both Entities and Resources directly
//          Entity
//          └── Manages its own Attributes
//          └── Resource

// =============== Scene Manager (Top Level) ===============
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SceneManager {
    pub scenes: IndexMap<Uuid, Scene>,
    pub shared_entities: IndexMap<Uuid, Entity>,
    pub active_scene: Option<Uuid>,  // Track currently active scene
}

impl SceneManager {
    pub fn new() -> Self {
        Self {
            scenes: IndexMap::new(),
            shared_entities: IndexMap::new(),
            active_scene: None,
        }
    }

    pub fn create_scene(&mut self, name: &str) -> Uuid {
        let id = Uuid::new_v4();
        let scene = Scene::new(name);
        self.scenes.insert(id, scene);
        id
    }

    pub fn delete_scene(&mut self, id: Uuid) -> bool {
        self.scenes.shift_remove(&id).is_some()
    }

    pub fn list_scene(&self) -> Vec<(Uuid, &str)> {
        self.scenes
            .iter()
            .map(|(id, scene)| (*id, scene.name.as_str()))
            .collect()
    }

    pub fn get_scene(&self, id: Uuid) -> Option<&Scene> {
        self.scenes.get(&id)
    }

    pub fn get_scene_mut(&mut self, id: Uuid) -> Option<&mut Scene> {
        self.scenes.get_mut(&id)
    }

    pub fn get_scene_by_name(&self, name: &str) -> Option<&Scene> {
        self.scenes
            .iter()
            .find(|(_, scene)| scene.name == name)
            .map(|(_, scene)| scene)
    }

    pub fn create_shared_entity(&mut self, name: &str) -> Uuid {
        let id = Uuid::new_v4();
        let entity = Entity::new(id, name);
        self.shared_entities.insert(id, entity);
        id
    }

    pub fn delete_shared_entity(&mut self, id: Uuid) -> bool {
        for scene in self.scenes.values_mut() {
            scene.shared_entity_refs.retain(|&ref_id| ref_id != id);
        }
        self.shared_entities.shift_remove(&id).is_some()
    }

    pub fn list_shared_entity(&self) -> Vec<(Uuid, &str)> {
        self.shared_entities
            .iter()
            .map(|(id, entity)| (*id, entity.name.as_str()))
            .collect()
    }

    pub fn get_shared_entity(&self, id: Uuid) -> Option<&Entity> {
        self.shared_entities.get(&id)
    }

    pub fn get_shared_entity_mut(&mut self, id: Uuid) -> Option<&mut Entity> {
        self.shared_entities.get_mut(&id)
    }

    pub fn get_shared_entity_by_name(&self, name: &str) -> Option<&Entity> {
        self.shared_entities
            .iter()
            .find(|(_, entity)| entity.name == name)
            .map(|(_, entity)| entity)
    }

    // Helper to get all scenes using a shared entity
    pub fn get_scenes_using_entity(&self, entity_id: Uuid) -> Vec<&Scene> {
        self.scenes
            .values()
            .filter(|scene| scene.shared_entity_refs.contains(&entity_id))
            .collect()
    }

    // Add these methods for active scene management
    pub fn set_active_scene(&mut self, id: Uuid) -> Result<(), String> {
        if self.scenes.contains_key(&id) {
            self.active_scene = Some(id);
            Ok(())
        } else {
            Err("Scene not found".to_string())
        }
    }

    pub fn get_active_scene(&self) -> Option<&Scene> {
        self.active_scene.and_then(|id| self.scenes.get(&id))
    }

    pub fn get_active_scene_mut(&mut self) -> Option<&mut Scene> {
        self.active_scene.and_then(|id| self.scenes.get_mut(&id))
    }

    pub fn clear_active_scene(&mut self) {
        self.active_scene = None;
    }
}

// =============== Scene (Manages Entities and Resources) ===============
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Scene {
    pub id: Uuid,
    pub name: String,
    pub entities: IndexMap<Uuid, Entity>,
    pub shared_entity_refs: Vec<Uuid>,
    pub default_camera: Option<Uuid>,
}

impl Scene {
    pub fn new(name: &str) -> Self {
        let mut scene = Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            entities: IndexMap::new(),
            shared_entity_refs: Vec::new(),
            default_camera: None,
        };

        // Create default camera
        let camera_id = scene.create_camera("main_camera");
        scene.default_camera = Some(camera_id);

        scene
    }

    // Scene operations
    pub fn modify_scene(&mut self, new_name: &str) {
        self.name = new_name.to_string();
    }

    // Entity management
    pub fn create_entity(&mut self, name: &str) -> Uuid {
        let id = Uuid::new_v4();
        let entity = Entity::new(id, name);
        self.entities.insert(id, entity);
        id
    }

    pub fn delete_entity(&mut self, id: Uuid) -> bool {
        self.entities.shift_remove(&id).is_some()
    }

    pub fn list_entity(&self) -> Vec<(Uuid, &str)> {
        self.entities
            .iter()
            .map(|(id, entity)| (*id, entity.name.as_str()))
            .collect()
    }

    pub fn get_entity(&self, id: Uuid) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_entity_mut(&mut self, id: Uuid) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    // Add methods to work with shared entities
    pub fn add_shared_entity_ref(&mut self, shared_entity_id: Uuid) {
        if !self.shared_entity_refs.contains(&shared_entity_id) {
            self.shared_entity_refs.push(shared_entity_id);
        }
    }

    pub fn remove_shared_entity_ref(&mut self, shared_entity_id: Uuid) {
        self.shared_entity_refs.retain(|&id| id != shared_entity_id);
    }

    pub fn list_shared_entity_ref(&self) -> Vec<Uuid> {
        self.shared_entity_refs.clone()
    }

    // Get shared entity reference through scene manager
    pub fn get_shared_entity_ref<'a>(&'a self, scene_manager: &'a SceneManager, id: Uuid) -> Option<&Entity> {
        if self.shared_entity_refs.contains(&id) {
            scene_manager.get_shared_entity(id)
        } else {
            None
        }
    }

    // Get shared entity reference mut through scene manager
    pub fn get_shared_entity_ref_mut<'a>(&'a self, scene_manager: &'a mut SceneManager, id: Uuid) -> Option<&mut Entity> {
        if self.shared_entity_refs.contains(&id) {
            scene_manager.get_shared_entity_mut(id)
        } else {
            None
        }
    }

    // Helper to get all entities (both local and shared)
    pub fn get_all_entities<'a>(&'a self, scene_manager: &'a SceneManager) -> Vec<&Entity> {
        let mut all_entities = Vec::new();
        all_entities.extend(self.entities.values());
        all_entities.extend(
            self.shared_entity_refs
                .iter()
                .filter_map(|id| scene_manager.get_shared_entity(*id))
        );
        all_entities
    }

    // Predefined: Camera Entity
    pub fn create_camera(&mut self, name: &str) -> Uuid {
        let id = Uuid::new_v4();
        let camera = Entity::new_camera(id, name);
        self.entities.insert(id, camera);
        id
    }

    // Predefined: Physical Entity
    pub fn create_physical_entity(
        &mut self, 
        name: &str,
        position: (f32, f32),
        physics: PhysicsProperties
    ) -> Uuid {
        let id = Uuid::new_v4();
        let entity = Entity::new_physical(id, name, position, physics);
        self.entities.insert(id, entity);
        id
    }

    pub fn update_entity_attributes(&mut self, updates: Vec<(Uuid, Uuid, AttributeValue)>) {
        // Updates is a vec of (entity_id, attribute_id, new_value)
        self.entities.par_iter_mut().for_each(|(entity_id, entity)| {
            // Get all updates for this entity
            updates.iter()
                .filter(|(id, _, _)| id == entity_id)
                .for_each(|(_, attr_id, new_value)| {
                    entity.modify_attribute(
                        *attr_id,
                        None,
                        None,
                        Some(new_value.clone())
                    );
                });
        });
    }
}

// =============== Entity (Manages Attributes) ===============
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entity {
    pub id: Uuid,
    pub name: String,
    pub attributes: IndexMap<Uuid, Attribute>,
    // Resource paths
    pub images: Vec<PathBuf>,      // Multiple images (sprites, textures)
    pub sounds: Vec<PathBuf>,      // Multiple sounds (effects, music)
    pub script: Option<PathBuf>,   // Single script per entity
}

impl Entity {
    pub fn new(id: Uuid, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            attributes: IndexMap::new(),
            images: Vec::new(),
            sounds: Vec::new(),
            script: None,
        }
    }

    pub fn change_entity_name(&mut self, new_name: &str) {
        self.name = new_name.to_string();
    }

    // Resource management methods
    pub fn add_image(&mut self, path: PathBuf) {
        if !self.images.contains(&path) {
            self.images.push(path);
        }
    }

    pub fn remove_image(&mut self, path: &PathBuf) {
        self.images.retain(|p| p != path);
    }

    pub fn add_sound(&mut self, path: PathBuf) {
        if !self.sounds.contains(&path) {
            self.sounds.push(path);
        }
    }

    pub fn remove_sound(&mut self, path: &PathBuf) {
        self.sounds.retain(|p| p != path);
    }

    pub fn set_script(&mut self, path: PathBuf) {
        self.script = Some(path);
    }

    pub fn remove_script(&mut self) {
        self.script = None;
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

    pub fn get_image(&self, index: usize) -> Option<&PathBuf> {
        self.images.get(index)
    }

    pub fn get_sound(&self, index: usize) -> Option<&PathBuf> {
        self.sounds.get(index)
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
    ) -> Uuid {
        let id = Uuid::new_v4();
        let attribute = Attribute {
            id,
            name: name.to_string(),
            data_type,
            value,
        };
        self.attributes.insert(id, attribute);
        id
    }

    pub fn delete_attribute(&mut self, id: Uuid) -> bool {
        self.attributes.shift_remove(&id).is_some()
    }

    pub fn list_attribute(&self) -> Vec<(Uuid, &str)> {
        self.attributes
            .iter()
            .map(|(id, attr)| (*id, attr.name.as_str()))
            .collect()
    }

    pub fn get_attribute(&self, id: Uuid) -> Option<&Attribute> {
        self.attributes.get(&id)
    }

    pub fn get_attribute_mut(&mut self, id: Uuid) -> Option<&mut Attribute> {
        self.attributes.get_mut(&id)
    }

    pub fn modify_attribute(
        &mut self,
        id: Uuid,
        new_name: Option<String>,
        new_type: Option<AttributeType>,
        new_value: Option<AttributeValue>,
    ) -> bool {
        if let Some(attr) = self.attributes.get_mut(&id) {
            if let Some(name) = new_name {
                attr.name = name;
            }
            if let Some(data_type) = new_type {
                attr.data_type = data_type;
            }
            if let Some(value) = new_value {
                attr.value = value;
            }
            true
        } else {
            false
        }
    }

    pub fn get_attribute_by_name(&self, name: &str) -> Option<&Attribute> {
        self.attributes
            .iter()
            .find(|(_, attr)| attr.name == name)
            .map(|(_, attr)| attr)
    }

    // Predefined: Camera Entity
    pub fn new_camera(id: Uuid, name: &str) -> Self {
        let mut entity = Self::new(id, name);
        
        // Add camera-specific attributes
        entity.create_attribute("position", AttributeType::Vector2, AttributeValue::Vector2(0.0, 0.0));
        entity.create_attribute("zoom", AttributeType::Float, AttributeValue::Float(1.0));
        entity.create_attribute("rotation", AttributeType::Float, AttributeValue::Float(0.0));
        entity.create_attribute("is_camera", AttributeType::Boolean, AttributeValue::Boolean(true));
        
        entity
    }

    // Predefined: Physical Entity
    pub fn new_physical(
        id: Uuid, 
        name: &str,
        position: (f32, f32),
        physics: PhysicsProperties
    ) -> Self {
        let mut entity = Self::new(id, name);
        
        // Add physics-specific attributes
        entity.create_attribute("position", AttributeType::Vector2, 
            AttributeValue::Vector2(position.0, position.1));
        entity.create_attribute("is_movable", AttributeType::Boolean, 
            AttributeValue::Boolean(physics.is_movable));
        entity.create_attribute("has_gravity", AttributeType::Boolean, 
            AttributeValue::Boolean(physics.affected_by_gravity));
        entity.create_attribute("creates_gravity", AttributeType::Boolean, 
            AttributeValue::Boolean(physics.creates_gravity));
        entity.create_attribute("has_collision", AttributeType::Boolean, 
            AttributeValue::Boolean(physics.has_collision));
        entity.create_attribute("friction", AttributeType::Float, 
            AttributeValue::Float(physics.friction));
        entity.create_attribute("restitution", AttributeType::Float, 
            AttributeValue::Float(physics.restitution));
        entity.create_attribute("density", AttributeType::Float, 
            AttributeValue::Float(physics.density));
        entity.create_attribute("can_rotate", AttributeType::Boolean, 
            AttributeValue::Boolean(physics.can_rotate));
        
        entity
    }
}

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
            },
            AttributeValue::String(value) => write!(f, "{}", value),
            AttributeValue::Boolean(value) => write!(f, "{}", value),
            AttributeValue::Vector2(x, y) => write!(f, "{}, {}", x, y),
        }
    }
}

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
