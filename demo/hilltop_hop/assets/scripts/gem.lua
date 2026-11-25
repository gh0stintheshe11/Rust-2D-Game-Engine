-- A gem: bobs up and down, and is collected when the player touches it
-- (gems are trigger zones: is_sensor).

local resting_y = {}

function init(scene_id, entity_id)
    resting_y[entity_id] = get_attribute(scene_id, entity_id, "y")
end

function update(scene_id, entity_id)
    local y = resting_y[entity_id]
    if y then
        -- Each gem at its own phase, from where it is
        local x = get_attribute(scene_id, entity_id, "x")
        set_y(scene_id, entity_id, y + math.sin(accumulated_time * 3 + x * 0.05) * 3)
    end
end

function on_collision(scene_id, entity_id, other_id)
    if get_entity_name(scene_id, other_id) ~= "player" then
        return
    end
    local state = script_state.state
    state.gems = state.gems + 1
    local x = get_attribute(scene_id, entity_id, "x")
    local y = get_attribute(scene_id, entity_id, "y")
    emit_particles(scene_id, find_entity(scene_id, "sparkle"), 14, x + 11, y + 13)
    play_sound("assets/sounds/gem.wav", { volume = 0.7 })
    remove_entity(scene_id, entity_id)
end
