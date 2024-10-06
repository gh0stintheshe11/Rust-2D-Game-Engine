# Rust 2D Game Engine

## Table of Contents

- [ECS (Entity Component System)](#ecs-entity-component-system)

### ECS (Entity Component System)

The [Entity Component System (ECS)](/src/ECS.rs) in this project serves as the core for managing game entities and their associated attributes. This section describes how the ECS system works, introduces key functions, and explains the behavior tested by our comprehensive unit tests.

#### Entity Manager

The EntityManager is responsible for creating, managing, and deleting entities. Each entity is represented by an Entity struct, which holds an ID and a set of attributes. Entities can have multiple attributes, each associated with a specific type (e.g., integer, float, string, or boolean).

#### Key Functions

	1.	create_entity
	•	Description: Creates a new entity with a unique ID and stores it in the EntityManager. The entity is returned for further manipulation.
	•	Usage:

```rust
let mut entity_manager = EntityManager::new();
let entity = entity_manager.create_entity();
```


	2.	delete_entity
	•	Description: Deletes an entity from the EntityManager.
	•	Usage:

entity_manager.delete_entity(entity);


	3.	copy_entity
	•	Description: Creates a copy of an existing entity, including all of its attributes. The new entity has a unique ID and independent attributes.
	•	Usage:

let copied_entity = entity_manager.copy_entity(&original_entity);


	4.	attribute_exists
	•	Description: Checks if a specific attribute exists for a given entity.
	•	Usage:

```rust
let exists = entity_manager.attribute_exists(&entity, &"Health".to_string());
```



#### Attribute Management

Each entity can have multiple attributes, which are represented as a key-value pair where the key is the attribute’s name, and the value is stored as an Attribute. Attributes are typed using the AttributeValueType enum, which supports the following types:

	•	Integer(i32)
	•	Float(f32)
	•	String(String)
	•	Boolean(bool)

#### Key Functions

1. add_attribute_with_validation
	•	Description: Adds an attribute to an entity, ensuring that the provided value matches the expected type. The function validates the input based on the attribute type before storing it.
	•	Usage:

```rust
entity_manager.add_attribute_with_validation(
    &mut entity, 
    "Health".to_string(), 
    AttributeValueType::Integer(0), 
    "100".to_string()
).unwrap();
```

2. modify_attribute_with_validation
	•	Description: Modifies the value of an existing attribute, validating the new value against the expected type.
	•	Usage:

```rust
entity_manager.modify_attribute_with_validation(
    &mut entity, 
    "Health".to_string(), 
    AttributeValueType::Integer(0), 
    "80".to_string()
).unwrap();
```

3. delete_attribute
	•	Description: Removes an attribute from an entity.
	•	Usage:

```rust
entity_manager.delete_attribute(&mut entity, &"Health".to_string());
```


4. validate_value
	•	Description: A helper function used internally to validate the type of an attribute value before storing or modifying it. It ensures that the provided value matches the expected type (e.g., an integer value for an integer type).
	•	Usage: This function is used internally by add_attribute_with_validation and modify_attribute_with_validation.

#### AttributeValueType Enum

The AttributeValueType enum defines the possible types for an entity’s attributes. It currently supports four types:

	•	Integer(i32)
	•	Float(f32)
	•	String(String)
	•	Boolean(bool)

#### [Unit Tests](/tests/ecs_test.rs)

We’ve included comprehensive unit tests to ensure the correctness of the ECS system. Below is an overview of each test and what it verifies:

1. add_valid_integer_attribute_test
	•	Verifies that a valid integer attribute can be added to an entity.
	•	Confirms that the entity contains the expected attribute after the operation.
2. add_invalid_integer_attribute_test
	•	Ensures that an invalid integer value (e.g., a string like "abc") is rejected when attempting to add an integer attribute.
3. add_valid_float_attribute_test
	•	Verifies that a valid float attribute can be added to an entity and correctly stored.
4. add_invalid_float_attribute_test
	•	Ensures that invalid float values are rejected when adding a float attribute.
5. modify_valid_integer_attribute_test
	•	Confirms that modifying an existing integer attribute with a valid new value works correctly.
6. modify_invalid_integer_attribute_test
	•	Ensures that attempting to modify an integer attribute with an invalid value (e.g., "abc") is rejected.
7. delete_attribute_test
	•	Verifies that an attribute can be successfully deleted from an entity and no longer exists afterward.
8. attribute_exists_test
	•	Confirms that checking for an attribute’s existence works correctly before and after adding an attribute.
9. copy_entity_test
	•	Verifies that copying an entity results in a new entity with the same attributes as the original.
	•	Ensures that modifying the copied entity does not affect the original and vice versa.

#### Example Usage

Here’s a basic example of how to use the ECS system:

```rust
fn main() {
    let mut entity_manager = EntityManager::new();
    
    // Create an entity
    let mut player = entity_manager.create_entity();
    
    // Add an attribute
    entity_manager.add_attribute_with_validation(
        &mut player, 
        "Health".to_string(), 
        AttributeValueType::Integer(0), 
        "100".to_string()
    ).unwrap();
    
    // Copy the entity
    let copied_player = entity_manager.copy_entity(&player);
    
    // Modify the copied entity
    entity_manager.modify_attribute_with_validation(
        &mut copied_player, 
        "Health".to_string(), 
        AttributeValueType::Integer(0), 
        "80".to_string()
    ).unwrap();
    
    // The original player's health remains unchanged
    assert_eq!(player.attributes.get("Health").unwrap().value_type, AttributeValueType::Integer(100));
    
    // The copied player's health is modified
    assert_eq!(copied_player.attributes.get("Health").unwrap().value_type, AttributeValueType::Integer(80));
}
```
