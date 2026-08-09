use super::{AttributeValue, Entity, PhysicsProperties, SceneManager};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub fn new(name: &str) -> Result<Self, String> {
        let mut scene = Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            entities: IndexMap::new(),
            shared_entity_refs: Vec::new(),
            default_camera: None,
        };

        // Create default camera
        let camera_id = scene.create_camera("main_camera")?;
        scene.default_camera = Some(camera_id);

        Ok(scene)
    }

    // Scene operations
    pub fn modify_scene(&mut self, new_name: &str) -> Result<(), String> {
        if new_name.is_empty() {
            return Err("Scene name cannot be empty".to_string());
        }
        self.name = new_name.to_string();
        Ok(())
    }

    // Entity management
    pub fn create_entity(&mut self, name: &str) -> Result<Uuid, String> {
        let id = Uuid::new_v4();
        let entity = Entity::new(id, name)?;
        self.entities.insert(id, entity);
        Ok(id)
    }

    pub fn delete_entity(&mut self, id: Uuid) -> Result<bool, String> {
        if Some(id) == self.default_camera {
            return Err("Cannot delete default camera".to_string());
        }
        Ok(self.entities.shift_remove(&id).is_some())
    }

    pub fn list_entity(&self) -> Vec<(Uuid, &str)> {
        self.entities
            .iter()
            .map(|(id, entity)| (*id, entity.name.as_str()))
            .collect()
    }

    pub fn get_entity(&self, id: Uuid) -> Result<&Entity, String> {
        self.entities
            .get(&id)
            .ok_or_else(|| format!("Entity {} not found", id))
    }

    pub fn get_entity_mut(&mut self, id: Uuid) -> Result<&mut Entity, String> {
        self.entities
            .get_mut(&id)
            .ok_or_else(|| format!("Entity {} not found", id))
    }

    // Add methods to work with shared entities
    pub fn add_shared_entity_ref(&mut self, shared_entity_id: Uuid) -> Result<(), String> {
        if self.shared_entity_refs.contains(&shared_entity_id) {
            Err("Shared entity reference already exists".to_string())
        } else {
            self.shared_entity_refs.push(shared_entity_id);
            Ok(())
        }
    }

    pub fn remove_shared_entity_ref(&mut self, shared_entity_id: Uuid) -> Result<(), String> {
        if !self.shared_entity_refs.contains(&shared_entity_id) {
            return Err("Shared entity reference not found".to_string());
        }
        self.shared_entity_refs.retain(|&id| id != shared_entity_id);
        Ok(())
    }

    pub fn list_shared_entity_ref(&self) -> Vec<Uuid> {
        self.shared_entity_refs.clone()
    }

    // Get shared entity reference through scene manager
    pub fn get_shared_entity_ref<'a>(
        &'a self,
        scene_manager: &'a SceneManager,
        id: Uuid,
    ) -> Option<&'a Entity> {
        if self.shared_entity_refs.contains(&id) {
            scene_manager.get_shared_entity(id)
        } else {
            None
        }
    }

    // Get shared entity reference mut through scene manager
    pub fn get_shared_entity_ref_mut<'a>(
        &'a self,
        scene_manager: &'a mut SceneManager,
        id: Uuid,
    ) -> Option<&'a mut Entity> {
        if self.shared_entity_refs.contains(&id) {
            scene_manager.get_shared_entity_mut(id)
        } else {
            None
        }
    }

    // Helper to get all entities (both local and shared)
    pub fn get_all_entities<'a>(&'a self, scene_manager: &'a SceneManager) -> Vec<&'a Entity> {
        let mut all_entities = Vec::new();
        all_entities.extend(self.entities.values());
        all_entities.extend(
            self.shared_entity_refs
                .iter()
                .filter_map(|id| scene_manager.get_shared_entity(*id)),
        );
        all_entities
    }

    // Predefined: Camera Entity
    pub fn create_camera(&mut self, name: &str) -> Result<Uuid, String> {
        let id = Uuid::new_v4();
        let camera = Entity::new_camera(id, name)?;
        self.entities.insert(id, camera);
        Ok(id)
    }

    // Predefined: Physical Entity
    pub fn create_physical_entity(
        &mut self,
        name: &str,
        position: (f32, f32, f32),
        physics: PhysicsProperties,
    ) -> Result<Uuid, String> {
        let id = Uuid::new_v4();
        let entity = Entity::new_physical(id, name, position, physics)?;
        self.entities.insert(id, entity);
        Ok(id)
    }

    pub fn update_entity_attributes(
        &mut self,
        updates: Vec<(Uuid, Uuid, AttributeValue)>,
    ) -> Result<(), String> {
        for (entity_id, attr_id, new_value) in updates {
            if let Some(entity) = self.entities.get_mut(&entity_id) {
                entity.modify_attribute(attr_id, None, None, Some(new_value.clone()))?;
            } else {
                return Err(format!("Entity {} not found", entity_id));
            }
        }
        Ok(())
    }

    pub fn update_entity_attribute(
        &mut self,
        entity_id: Uuid,
        attr_id: Uuid,
        new_value: AttributeValue,
    ) -> Result<(), String> {
        if let Some(entity) = self.entities.get_mut(&entity_id) {
            entity.modify_attribute(attr_id, None, None, Some(new_value.clone()))?;
        } else {
            return Err(format!("Entity {} not found", entity_id));
        }
        Ok(())
    }
}
