-- The title screen: shows the best results, kept between runs of the game
-- (save_data / load_data), plays the title music, and starts the level.

local CENTER = 480
local MUSIC = "assets/sounds/title_music.wav"

function init(scene_id, entity_id)
    -- Every track plays in the "music" volume group, which the player can
    -- turn off; the setting is kept between runs
    set_group_volume("music", load_data("music_off") and 0 or 1)
    -- Coming from the level, its music fades out as this one fades in
    play_music(MUSIC, { volume = 0.5, crossfade = 0.8 })
end

function update(scene_id, entity_id)
    draw_rect(270, 50, 420, 300, { color = "#10182cb0", radius = 18 })
    draw_text("Hilltop Hop", CENTER, 70, { size = 60, color = "#ffe27a", align = "center" })
    draw_text("Collect the gems and reach the flag", CENTER, 150, { size = 24, align = "center" })

    local best_time = load_data("best_time")
    if best_time then
        local best = string.format("Best: %.1f s, %d / 15 gems", best_time, load_data("best_gems") or 0)
        draw_text(best, CENTER, 186, { size = 22, color = "#c8f0ff", align = "center" })
    end

    -- The default button: Enter and gamepad A press it right away; so do
    -- the jump and menu actions
    if button("Play", CENTER - 90, 228, 180, 58, { size = 30, default = true })
        or is_action_just_pressed("jump") or is_action_just_pressed("menu") then
        change_scene("level", { transition = "fade", seconds = 0.5 })
    end
    draw_text("Run: arrow keys, A / D, stick or d-pad", CENTER, 300, { size = 18, color = "#dde6f5", align = "center" })
    draw_text("Jump: Space, W, Up or gamepad A", CENTER, 322, { size = 18, color = "#dde6f5", align = "center" })

    -- Music on or off: a click, or the "music" action (M or gamepad Y). The
    -- button never takes the keyboard and gamepad focus, which stays on Play
    local on = get_group_volume("music") > 0
    if button(on and "Music: on (M)" or "Music: off (M)", 790, 482, 150, 38,
            { size = 18, focusable = false })
        or is_action_just_pressed("music") then
        set_group_volume("music", on and 0 or 1)
        save_data("music_off", on)
    end
end
