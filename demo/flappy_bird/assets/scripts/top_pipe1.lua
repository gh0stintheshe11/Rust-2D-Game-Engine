function update(scene_id, entity_id)
    -- One-time migration: make this pipe kinematic so the bird can't push it.
    -- pcall tolerates pipes that already have the attribute (spawned ones).
    local key = "pipe_kinematic_" .. entity_id
    if script_state.state[key] == nil then
        script_state.state[key] = true
        pcall(create_attribute_bool, scene_id, entity_id, "is_kinematic", true)
        -- Rebuild the physics body with the new type (re-add replaces it)
        add_entity_to_physics_engine(entity_id)
    end

    local velocity_x = -50.0
    local velocity_y = 0.0

    set_velocity(entity_id, velocity_x, velocity_y)
end
