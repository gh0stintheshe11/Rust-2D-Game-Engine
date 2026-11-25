-- A checkpoint: once the player touches it, the flag turns green (frame 1
-- of its sprite sheet) and the player comes back here after losing a heart.

local FLAG_HEIGHT, PLAYER_HEIGHT = 56, 42

function on_collision(scene_id, entity_id, other_id)
    if get_entity_name(scene_id, other_id) ~= "player" then
        return
    end
    if get_attribute(scene_id, entity_id, "frame") == 1 then
        return
    end
    set_attribute(scene_id, entity_id, "frame", 1)
    local x = get_attribute(scene_id, entity_id, "x")
    local y = get_attribute(scene_id, entity_id, "y")
    -- Where the player stands next to the flag
    script_state.state.checkpoint = { x = x, y = y + FLAG_HEIGHT - PLAYER_HEIGHT - 2 }
    emit_particles(scene_id, find_entity(scene_id, "sparkle"), 20, x + 12, y + 12)
    play_sound("assets/sounds/checkpoint.wav", { volume = 0.7 })
end
