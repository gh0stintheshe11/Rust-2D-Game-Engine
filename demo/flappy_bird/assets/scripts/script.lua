function update(scene_id, entity_id)
    local force_x = 0.0

    local script_key = "bird"
    if script_state["state"][script_key] == nil then
        script_state["state"][script_key] = { is_just_jumped = false, jump_count = 0.0 }
    end
    local state = script_state["state"][script_key]

    if is_key_just_pressed("Space") then
        state.is_just_jumped = true
        state.jump_count = 15.0
        set_velocity(entity_id, force_x, -100.0)
    elseif state.is_just_jumped and state.jump_count <= 0.0 then
        state.is_just_jumped = false
        set_velocity(entity_id, force_x, 0.0)
    end

    if state.jump_count > 0.0 then
        state.jump_count = state.jump_count - 1.0
    end

    -- Game over when the bird touches a pipe or the ground
    local colliding = get_colliding_entities(entity_id)
    for i = 1, #colliding do
        local name = get_entity_name(scene_id, colliding[i])
        if name ~= nil then
            if string.sub(name, 1, 8) == "top_pipe"
                or string.sub(name, 1, 11) == "bottom_pipe"
                or string.sub(name, 1, 6) == "ground" then
                end_game()
                return
            end
        end
    end
end
