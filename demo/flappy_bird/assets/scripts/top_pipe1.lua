function update(scene_id, entity_id)
    -- One-time migration: make this pipe kinematic so the bird can't push it.
    if not has_attribute(scene_id, entity_id, "is_kinematic") then
        create_attribute_bool(scene_id, entity_id, "is_kinematic", true)
        -- Rebuild the physics body with the new type (re-add replaces it)
        add_entity_to_physics_engine(entity_id)
    end

    local velocity_x = -50.0
    local velocity_y = 0.0

    set_velocity(entity_id, velocity_x, velocity_y)
end
