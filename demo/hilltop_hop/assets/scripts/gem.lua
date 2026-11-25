-- A gem: bobs up and down, and is collected when the player touches it
-- (gems are trigger zones: is_sensor).

-- One way up or down (a tween back and forth), in seconds
local BOB_TIME = 1.05

function init(scene_id, entity_id)
    -- Each gem at its own phase, from where it is
    local x = get_attribute(scene_id, entity_id, "x")
    local y = get_attribute(scene_id, entity_id, "y")
    tween(entity_id, { y = y - 6 }, BOB_TIME,
        { ease = "sine_in_out", loop = "ping_pong", delay = (x * 0.017) % (2 * BOB_TIME) })
end

function on_collision(scene_id, entity_id, other_id)
    if get_entity_name(scene_id, other_id) ~= "player" then
        return
    end
    local state = script_state.state
    state.gems = state.gems + 1
    remove_entity(scene_id, entity_id)
end

-- Runs as the gem is removed, while it's still there: a sparkle where it was
function on_destroy(scene_id, entity_id)
    local x = get_attribute(scene_id, entity_id, "x")
    local y = get_attribute(scene_id, entity_id, "y")
    emit_particles(scene_id, find_entity(scene_id, "sparkle"), 14, x + 11, y + 13)
    play_sound("assets/sounds/gem.wav", { volume = 0.7 })
end
