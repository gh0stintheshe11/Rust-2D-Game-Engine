use std::collections::HashMap;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

//SceneManager
// └── Manages multiple Scenes
//      Scene
//      └── Manages both Entities and Resources directly
//          Entity
//          └── Manages its own Attributes
//          Resource
//          └── (Has its specific operations like play/display/edit)

// =============== Scene Manager (Top Level) ===============
pub struct SceneManager {
    scenes: HashMap<Uuid, Scene>,
}

impl SceneManager {
    pub fn new() -> Self {
        Self {
            scenes: HashMap::new(),
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
}

// =============== Scene (Manages Entities and Resources) ===============
#[derive(Serialize, Deserialize, Debug)]
pub struct Scene {
    pub id: Uuid,
    pub name: String,
    pub entities: HashMap<Uuid, Entity>,
    pub resources: HashMap<Uuid, Resource>,
}

impl Scene {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            entities: HashMap::new(),
            resources: HashMap::new(),
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AttributeValue {
    Integer(i32),
    Float(f32),
    String(String),
    Boolean(bool),
    Vector2(f32, f32),
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
