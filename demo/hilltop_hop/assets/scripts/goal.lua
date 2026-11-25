-- The goal: touching the flag wins the level, and keeps the best results
-- between runs of the game (save_data / load_data).

function on_collision(scene_id, entity_id, other_id)
    local state = script_state.state
    if state.won or state.game_over or get_entity_name(scene_id, other_id) ~= "player" then
        return
    end
    state.won = true
    state.ended_at = accumulated_time

    if state.gems > (load_data("best_gems") or 0) then
        save_data("best_gems", state.gems)
    end
    local best_time = load_data("best_time")
    state.new_best_time = best_time == nil or state.time < best_time
    if state.new_best_time then
        save_data("best_time", state.time)
    end

    local x = get_attribute(scene_id, entity_id, "x")
    local y = get_attribute(scene_id, entity_id, "y")
    emit_particles(scene_id, find_entity(scene_id, "confetti"), 90, x + 26, y + 20)
    play_sound("assets/sounds/win.wav")
end
