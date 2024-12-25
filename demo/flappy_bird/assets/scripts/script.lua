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

    if is_key_just_pressed("Space") then
        apply_impulse(entity_id, force_x, force_y)
    	print("A key was just pressed!")
	end

    --print("Force set to (" .. force_x .. ", " .. force_y .. ") for entity " .. entity_id)
end