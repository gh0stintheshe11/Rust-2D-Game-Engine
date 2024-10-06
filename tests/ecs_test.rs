// Import the ECS module from the main project
use rust_2d_game_engine::ecs::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_valid_integer_attribute_test() {
        let mut entity_manager = EntityManager::new();
        let mut entity = entity_manager.create_entity();
        let result = entity_manager.add_attribute_with_validation(
            &mut entity,
            "Health".to_string(),
            AttributeValueType::Integer(0),
            "100".to_string(),
        );
        assert!(result.is_ok());
        if let Some(attr) = entity.attributes.get(&"Health".to_string()) {
            assert_eq!(attr.value_type, AttributeValueType::Integer(100));
        }
    }

    #[test]
    fn add_invalid_integer_attribute_test() {
        let mut entity_manager = EntityManager::new();
        let mut entity = entity_manager.create_entity();
        let result = entity_manager.add_attribute_with_validation(
            &mut entity,
            "Health".to_string(),
            AttributeValueType::Integer(0),
            "abc".to_string(),
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid integer value: abc");
    }

    #[test]
    fn add_valid_float_attribute_test() {
        let mut entity_manager = EntityManager::new();
        let mut entity = entity_manager.create_entity();
        let result = entity_manager.add_attribute_with_validation(
            &mut entity,
            "Speed".to_string(),
            AttributeValueType::Float(0.0),
            "2.5".to_string(),
        );
        assert!(result.is_ok());
        if let Some(attr) = entity.attributes.get(&"Speed".to_string()) {
            assert_eq!(attr.value_type, AttributeValueType::Float(2.5));
        }
    }

    #[test]
    fn add_invalid_float_attribute_test() {
        let mut entity_manager = EntityManager::new();
        let mut entity = entity_manager.create_entity();
        let result = entity_manager.add_attribute_with_validation(
            &mut entity,
            "Speed".to_string(),
            AttributeValueType::Float(0.0),
            "abc".to_string(),
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid float value: abc");
    }

    #[test]
    fn modify_valid_integer_attribute_test() {
        let mut entity_manager = EntityManager::new();
        let mut entity = entity_manager.create_entity();
        entity_manager
            .add_attribute_with_validation(
                &mut entity,
                "Health".to_string(),
                AttributeValueType::Integer(0),
                "100".to_string(),
            )
            .unwrap();

        let modify_result = entity_manager.modify_attribute_with_validation(
            &mut entity,
            "Health".to_string(),
            AttributeValueType::Integer(0),
            "80".to_string(),
        );
        assert!(modify_result.is_ok());
        if let Some(attr) = entity.attributes.get(&"Health".to_string()) {
            assert_eq!(attr.value_type, AttributeValueType::Integer(80));
        }
    }

    #[test]
    fn modify_invalid_integer_attribute_test() {
        let mut entity_manager = EntityManager::new();
        let mut entity = entity_manager.create_entity();
        entity_manager
            .add_attribute_with_validation(
                &mut entity,
                "Health".to_string(),
                AttributeValueType::Integer(0),
                "100".to_string(),
            )
            .unwrap();

        let modify_result = entity_manager.modify_attribute_with_validation(
            &mut entity,
            "Health".to_string(),
            AttributeValueType::Integer(0),
            "abc".to_string(),
        );
        assert!(modify_result.is_err());
        assert_eq!(modify_result.unwrap_err(), "Invalid integer value: abc");
    }

    #[test]
    fn delete_attribute_test() {
        let mut entity_manager = EntityManager::new();
        let mut entity = entity_manager.create_entity();
        entity_manager
            .add_attribute_with_validation(
                &mut entity,
                "Health".to_string(),
                AttributeValueType::Integer(0),
                "100".to_string(),
            )
            .unwrap();

        // Deleting the attribute
        entity_manager.delete_attribute(&mut entity, &"Health".to_string());

        assert!(entity.attributes.get(&"Health".to_string()).is_none());
    }

    #[test]
    fn modify_clone_and_check_original_unchanged_test() {
        let mut entity_manager = EntityManager::new();
        let entity = entity_manager.create_entity();
        let mut cloned_entity = entity.clone();

        // Add attribute to the cloned entity
        entity_manager
            .add_attribute_with_validation(
                &mut cloned_entity,
                "Health".to_string(),
                AttributeValueType::Integer(0),
                "100".to_string(),
            )
            .unwrap();

        // Check if the cloned entity has the "Health" attribute
        assert!(cloned_entity.attributes.contains_key("Health"));

        // Check that the original entity does NOT have the "Health" attribute
        assert!(!entity.attributes.contains_key("Health"));
    }

    #[test]
    fn modify_original_and_check_clone_unchanged_test() {
        let mut entity_manager = EntityManager::new();
        let mut entity = entity_manager.create_entity();
        let cloned_entity = entity.clone();

        // Add attribute to the original entity
        entity_manager
            .add_attribute_with_validation(
                &mut entity,
                "Health".to_string(),
                AttributeValueType::Integer(0),
                "100".to_string(),
            )
            .unwrap();

        // Check if the original entity has the "Health" attribute
        assert!(entity.attributes.contains_key("Health"));

        // Check that the cloned entity does NOT have the "Health" attribute
        assert!(!cloned_entity.attributes.contains_key("Health"));
    }

    #[test]
    fn copy_entity_test() {
        let mut entity_manager = EntityManager::new();
        
        // Create original entity and add attributes
        let mut original_entity = entity_manager.create_entity();
        entity_manager
            .add_attribute_with_validation(
                &mut original_entity,
                "Health".to_string(),
                AttributeValueType::Integer(0),
                "100".to_string(),
            )
            .unwrap();

        // Copy the original entity
        let mut copied_entity = entity_manager.copy_entity(&original_entity);

        // Check that both original and copied entities have the same "Health" attribute
        assert!(original_entity.attributes.contains_key("Health"));
        assert!(copied_entity.attributes.contains_key("Health"));

        // Verify that the values of the "Health" attribute are the same
        assert_eq!(
            original_entity.attributes.get("Health").unwrap().value_type,
            AttributeValueType::Integer(100)
        );
        assert_eq!(
            copied_entity.attributes.get("Health").unwrap().value_type,
            AttributeValueType::Integer(100)
        );

        // Modify the copied entity's attribute and check that the original is unchanged
        entity_manager
            .modify_attribute_with_validation(
                &mut copied_entity,
                "Health".to_string(),
                AttributeValueType::Integer(0),
                "80".to_string(),
            )
            .unwrap();

        // Check that the copied entity's attribute has changed
        assert_eq!(
            copied_entity.attributes.get("Health").unwrap().value_type,
            AttributeValueType::Integer(80)
        );

        // Check that the original entity's attribute remains unchanged
        assert_eq!(
            original_entity.attributes.get("Health").unwrap().value_type,
            AttributeValueType::Integer(100)
        );
    }
}