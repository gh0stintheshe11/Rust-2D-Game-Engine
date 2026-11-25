-- Attached to the star and rock prefabs, so to every falling star and rock.

function on_collision(scene_id, entity_id, other_id)
    local state = script_state.state
    local kind = get_entity_name(scene_id, entity_id)
    local other = get_entity_name(scene_id, other_id)

    if other == "paddle" and not state.game_over then
        if kind == "star" then
            play_sound("assets/sounds/catch.wav")
            state.score = state.score + 1
            remove_entity(scene_id, entity_id)
        else
            play_sound("assets/sounds/crash.wav")
            state.game_over = true
            state.best = math.max(state.best or 0, state.score)
            save_data("best", state.best)
        end
    elseif other == "ground" or other == "paddle" then
        -- Missed, or the round is over: it just disappears
        remove_entity(scene_id, entity_id)
    end
end
