-- Attached to every falling star and rock by the spawner.

function on_collision(scene_id, entity_id, other_id)
    local kind = get_entity_name(scene_id, entity_id)
    local other = get_entity_name(scene_id, other_id)

    if other == "paddle" then
        if kind == "star" then
            play_sound("assets/sounds/catch.wav")
            local state = script_state.state
            state.score = state.score + 1
            remove_entity(scene_id, entity_id)
        else
            play_sound("assets/sounds/crash.wav")
            print("Game over! Stars caught: " .. script_state.state.score)
            end_game()
        end
    elseif other == "ground" then
        -- Missed: it just disappears
        remove_entity(scene_id, entity_id)
    end
end
