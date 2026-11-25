-- The title screen: shows the best results, kept between runs of the game
-- (save_data / load_data), and starts the level.

local CENTER = 480

function update(scene_id, entity_id)
    draw_rect(270, 50, 420, 300, { color = "#10182cb0", radius = 18 })
    draw_text("Hilltop Hop", CENTER, 70, { size = 60, color = "#ffe27a", align = "center" })
    draw_text("Collect the gems and reach the flag", CENTER, 150, { size = 24, align = "center" })

    local best_time = load_data("best_time")
    if best_time then
        local best = string.format("Best: %.1f s, %d / 15 gems", best_time, load_data("best_gems") or 0)
        draw_text(best, CENTER, 186, { size = 22, color = "#c8f0ff", align = "center" })
    end

    -- The default button: Enter and gamepad A press it right away
    if button("Play", CENTER - 90, 228, 180, 58, { size = 30, default = true })
        or is_key_just_pressed("Space") or is_gamepad_just_pressed("start") then
        change_scene("level")
    end
    draw_text("Run: arrow keys, A / D, stick or d-pad", CENTER, 300, { size = 18, color = "#dde6f5", align = "center" })
    draw_text("Jump: Space, W, Up or gamepad A", CENTER, 322, { size = 18, color = "#dde6f5", align = "center" })
end
