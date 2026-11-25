-- The title screen: starts the game, and shows the best score, which is
-- saved between runs of the game (save_data / load_data) and carried in
-- script_state while it runs.

function init(scene_id, entity_id)
    if script_state.state.best == nil then
        script_state.state.best = load_data("best")
    end
end

function update(scene_id, entity_id)
    draw_text("Star Catcher", 240, 140, {size = 60, color = "#ffd84a", align = "center"})
    draw_text("Catch the stars, dodge the rocks", 240, 220, {size = 24, align = "center"})

    local best = script_state.state.best
    if best then
        draw_text("Best: " .. best, 240, 410, {size = 28, align = "center"})
    end
    draw_text("Press Space or (A) to play", 240, 480, {size = 30, color = "#80ff80", align = "center"})
    draw_text("Arrow keys, A / D or a gamepad to move", 240, 560, {size = 20, color = "#c8c8d8", align = "center"})

    if is_action_just_pressed("start") then
        -- The night sky wipes down over the title, like a falling star
        change_scene("main", {transition = "wipe_down", seconds = 0.4})
    end
end
