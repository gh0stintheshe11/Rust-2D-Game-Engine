-- The title screen: starts the game, and shows the best score of the
-- session (script_state carries over when scenes change).

function update(scene_id, entity_id)
    draw_text("Star Catcher", 240, 140, {size = 60, color = "#ffd84a", align = "center"})
    draw_text("Catch the stars, dodge the rocks", 240, 220, {size = 24, align = "center"})

    local best = script_state.state.best
    if best then
        draw_text("Best: " .. best, 240, 410, {size = 28, align = "center"})
    end
    draw_text("Press Space to play", 240, 480, {size = 30, color = "#80ff80", align = "center"})
    draw_text("Arrow keys or A / D to move", 240, 560, {size = 20, color = "#c8c8d8", align = "center"})

    if is_key_just_pressed("Space") or is_key_just_pressed("Enter") then
        change_scene("main")
    end
end
