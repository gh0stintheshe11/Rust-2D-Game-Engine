-- Generate random name for pipes
function generate_random_name(prefix)
    local random_number = math.random(1, 100000) -- Generate a random number
    return prefix .. tostring(random_number) -- Combine the prefix with the random number
end

-- Create predefined attributes for physics entity (tried create in Rust but not working)
function create_physics_attributes(scene_id, entity_id, x, y)
    --print("Creating predefined physics attributes for entity " .. entity_id .. " in scene " .. scene_id)

    -- Create Vector2 attribute for position
    create_attribute_vector2(scene_id, entity_id, "position", x, y)

    -- Create Boolean attributes
    create_attribute_bool(scene_id, entity_id, "is_movable", true)
    create_attribute_bool(scene_id, entity_id, "has_gravity", true)
    create_attribute_bool(scene_id, entity_id, "creates_gravity", false)
    create_attribute_bool(scene_id, entity_id, "has_collision", true)
    create_attribute_bool(scene_id, entity_id, "can_rotate", true)

    -- Create Float attributes
    create_attribute_float(scene_id, entity_id, "friction", 0.5)
    create_attribute_float(scene_id, entity_id, "restitution", 0.0)
    create_attribute_float(scene_id, entity_id, "density", 1.0)

    --print("Predefined physics attributes created for entity " .. entity_id)
end

-- Create pipe entity
function create_pipe(scene_id, pipe_name_prefix, x, y, image_path, script_path)
    -- Generate a random name for the pipe
    local pipe_name = generate_random_name(pipe_name_prefix)
    --print("Creating pipe '" .. pipe_name .. "' in scene " .. scene_id)

    --print("x " .. x, ", y " .. y)

    -- Add the pipe entity to the scene
    local entity_id = add_entity(scene_id, pipe_name)
    --print("Created pipe entity with ID: " .. entity_id)

    -- Update attributes for the pipe
    set_x(scene_id, entity_id, x)
    set_y(scene_id, entity_id, y)
    set_z(scene_id, entity_id, 1.0)
    set_position(scene_id, entity_id, x, y)

    -- Attach image and script
    add_image(entity_id, image_path)
    set_script(entity_id, script_path)

    --print("Pipe '" .. pipe_name .. "' created with attributes and assets.")
    return entity_id
end

-- TODO: due to list_entities_name_x_y is not working, this also not working
function cleanup_pipes(scene_id)
    local entities = list_entities_name_x_y(scene_id)
    for i = 1, #entities do
        local entity = entities[i]
        local entity_id = entity.id
        local name = entity.name
        local x = entity.x
        local y = entity.y
        print(name)
        if string.sub(name, 1, 9) == "top_pipe_" and x < -30 then
            remove_entity(scene_id, entity_id)
            remove_entity_from_physics_engine(entity_id)
            print("Removed entity: " .. name .. " with ID: " .. entity_id)
        end

        if string.sub(name, 1, 9) == "top_pipe_" and x < -30 then
            remove_entity(scene_id, entity_id)
            remove_entity_from_physics_engine(entity_id)
            print("Removed entity: " .. name .. " with ID: " .. entity_id)
        end
    end
end

print(accumulated_time);

-- main entry point for Rust to call
function update(scene_id, entity_id)
    --print("Updating entity " .. entity_id .. " in scene " .. scene_id)

    local script_key = "pipe_spawner"

    if script_state["state"][script_key] == nil then
        script_state["state"][script_key] = { last_trigger_time = 0.0 }
    end
    print("last_trigger_time:" .. script_state["state"][script_key].last_trigger_time)

    local state = script_state["state"][script_key]

    -- Time threshold for triggering (in seconds)
    local time_interval =5.0

    if accumulated_time - state.last_trigger_time >= time_interval then
        state.last_trigger_time = accumulated_time

        -- Generate random x and y positions
        local random_x = math.random(300, 400)
        local random_top_y = math.random(-100, -50) -- above top
        local random_bottom_y = math.random(150, 200) -- at least below top pipe, otherwise they hit each other and stop outside of the scene

        -- Create top pipe
        local top_pipe_id = create_pipe(
            scene_id,
            "top_pipe_",     -- Prefix for the pipe name
            random_x,        -- Random x position
            random_top_y,    -- Random y position
            "assets/images/top_pipe.png",  -- image path
            "assets/scripts/top_pipe1.lua" -- script path
        )

        local bottom_pipe_id = create_pipe(
                    scene_id,
                    "top_pipe_",
                    random_x,
                    random_bottom_y,
                    "assets/images/bottom_pipe.png",
                    "assets/scripts/top_pipe1.lua"
                )

        create_physics_attributes(scene_id, top_pipe_id, random_x, random_top_y)
        create_physics_attributes(scene_id, bottom_pipe_id, random_x, random_bottom_y)
        cleanup_pipes(scene_id)

        ---- Add entity to physics engine, due to it has different frame rate
        add_entity_to_physics_engine(top_pipe_id)
        add_entity_to_physics_engine(bottom_pipe_id)
        ----print("top pipe new id: " .. top_pipe_id .. ", x: " .. random_x .. ", y: " .. random_y)
        --
        --
        --local velocity_x = -10.0
        --local velocity_y = 0.0
        --
        --set_velocity(top_pipe_id, velocity_x, velocity_y)
        --set_velocity(bottom_pipe_id, velocity_x, velocity_y)


    end
end
