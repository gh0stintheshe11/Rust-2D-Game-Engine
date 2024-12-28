function update(scene_id, entity_id)
    --print("Updating entity " .. entity_id .. " in scene " .. scene_id)

    local force_x = 0.0
    local force_y = -10000.0

    --set_velocity(entity_id, velocity_x, velocity_y)

    --if keys_pressed then
    --    for _, key in ipairs(keys_pressed) do
    --        if key == "Space" then
    --            apply_impulse(entity_id, force_x, force_y)
    --        end
    --    end
    --end

    local script_key = "bird"
    if script_state["state"][script_key] == nil then
        script_state["state"][script_key] = { is_just_jumped = false, jump_count = 0.0 }
    end
    local state = script_state["state"][script_key]

    if is_key_just_pressed("Space") then
        state.is_just_jumped = true
        state.jump_count = 15.0
        set_velocity(entity_id, force_x, -100.0)
    	print("A key was just pressed!")
    elseif state.is_just_jumped and state.jump_count <= 0.0 then
        state.is_just_jumped = false
        set_velocity(entity_id, force_x, 0.0)
	end

	if state.jump_count > 0.0 then
	    state.jump_count = state.jump_count - 1.0
	end

    --print("Force set to (" .. force_x .. ", " .. force_y .. ") for entity " .. entity_id)
end