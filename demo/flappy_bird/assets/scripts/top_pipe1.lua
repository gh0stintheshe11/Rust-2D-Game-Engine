function update(scene_id, entity_id)
    --print("Updating entity " .. entity_id .. " in scene " .. scene_id)

    local velocity_x = -100.0
    local velocity_y = 0.0

    set_velocity(entity_id, velocity_x, velocity_y)

    --print("Velocity set to (" .. velocity_x .. ", " .. velocity_y .. ") for entity " .. entity_id)
end