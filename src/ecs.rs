use std::collections::HashMap;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use std::fmt;

//SceneManager
// └── Manages multiple Scenes
//      Scene
//      └── Manages both Entities and Resources directly
//          Entity
//          └── Manages its own Attributes
//          Resource
//          └── (Has its specific operations like play/display/edit)

// =============== Scene Manager (Top Level) ===============
#[derive(Serialize, Deserialize)]
pub struct SceneManager {
    pub scenes: HashMap<Uuid, Scene>,
    pub shared_entities: HashMap<Uuid, Entity>,
    pub active_scene: Option<Uuid>,  // Track currently active scene
}

impl SceneManager {
    pub fn new() -> Self {
        Self {
            scenes: HashMap::new(),
            shared_entities: HashMap::new(),
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
        self.scenes.remove(&id).is_some()
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
        self.shared_entities.remove(&id).is_some()
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
    pub entities: HashMap<Uuid, Entity>,
    pub resources: HashMap<Uuid, Resource>,
    pub shared_entity_refs: Vec<Uuid>,
}

impl Scene {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            entities: HashMap::new(),
            resources: HashMap::new(),
            shared_entity_refs: Vec::new(),
        }
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
        self.entities.remove(&id).is_some()
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

    // Resource management
    pub fn create_resource(
        &mut self,
        name: &str,
        file_path: &str,
        resource_type: ResourceType,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let resource = Resource {
            id,
            name: name.to_string(),
            file_path: file_path.to_string(),
            resource_type,
        };
        self.resources.insert(id, resource);
        id
    }

    pub fn delete_resource(&mut self, id: Uuid) -> bool {
        self.resources.remove(&id).is_some()
    }

    pub fn list_resource(&self) -> Vec<(Uuid, &str)> {
        self.resources
            .iter()
            .map(|(id, res)| (*id, res.name.as_str()))
            .collect()
    }

    pub fn get_resource(&self, id: Uuid) -> Option<&Resource> {
        self.resources.get(&id)
    }

    pub fn get_resource_mut(&mut self, id: Uuid) -> Option<&mut Resource> {
        self.resources.get_mut(&id)
    }

    pub fn modify_resource(
        &mut self,
        id: Uuid,
        new_name: Option<String>,
        new_path: Option<String>,
        new_type: Option<ResourceType>,
    ) -> bool {
        if let Some(resource) = self.resources.get_mut(&id) {
            if let Some(name) = new_name {
                resource.name = name;
            }
            if let Some(path) = new_path {
                resource.file_path = path;
            }
            if let Some(res_type) = new_type {
                resource.resource_type = res_type;
            }
            true
        } else {
            false
        }
    }

    pub fn get_resource_by_name(&self, name: &str) -> Option<&Resource> {
        self.resources
            .iter()
            .find(|(_, res)| res.name == name)
            .map(|(_, res)| res)
    }

    pub fn get_entity_by_name(&self, name: &str) -> Option<&Entity> {
        self.entities
            .iter()
            .find(|(_, ent)| ent.name == name)
            .map(|(_, ent)| ent)
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
}

// =============== Entity (Manages Attributes) ===============
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entity {
    pub id: Uuid,
    pub name: String,
    pub attributes: HashMap<Uuid, Attribute>,
    pub resource_list: Vec<Uuid>,
}

impl Entity {
    pub fn new(id: Uuid, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            attributes: HashMap::new(),
            resource_list: Vec::new(),
        }
    }

    pub fn change_entity_name(&mut self, new_name: &str) {
        self.name = new_name.to_string();
    }

    pub fn attach_resource(&mut self, resource_id: Uuid) {
        if !self.resource_list.contains(&resource_id) {
            self.resource_list.push(resource_id);
        }
    }

    pub fn detach_resource(&mut self, resource_id: Uuid) {
        self.resource_list.retain(|&id| id != resource_id);
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
        self.attributes.remove(&id).is_some()
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

// =============== Resource ===============
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Resource {
    pub id: Uuid,
    pub name: String,
    pub file_path: String,
    pub resource_type: ResourceType,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ResourceType {
    Image,
    Sound,
    Script,
}

impl Resource {
    pub fn display(&self) {
        match self.resource_type {
            ResourceType::Image => println!("Displaying image: {}", self.file_path),
            _ => println!("Cannot display this resource type"),
        }
    }

    pub fn play(&self) {
        match self.resource_type {
            ResourceType::Sound => println!("Playing sound: {}", self.file_path),
            _ => println!("Can only play sound resources"),
        }
    }

    pub fn pause(&self) {
        match self.resource_type {
            ResourceType::Sound => println!("Pausing sound: {}", self.file_path),
            _ => println!("Can only pause sound resources"),
        }
    }

    pub fn stop(&self) {
        match self.resource_type {
            ResourceType::Sound => println!("Stopping sound: {}", self.file_path),
            _ => println!("Can only stop sound resources"),
        }
    }

    pub fn edit(&self) {
        match self.resource_type {
            ResourceType::Script => println!("Editing script: {}", self.file_path),
            _ => println!("Can only edit script resources"),
        }
    }
}
