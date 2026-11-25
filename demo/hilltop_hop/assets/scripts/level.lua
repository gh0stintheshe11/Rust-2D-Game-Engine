-- The level: starts the round, has the camera follow the player (the sky
-- moves along, the hills at half speed for depth), keeps time, and draws
-- the heads-up display and the end screens.

local TILE = 32
local LEVEL_WIDTH = 110 * TILE
local VIEW_WIDTH, VIEW_HEIGHT = 960, 540
local HILLS_Y = 240
local GEMS_TOTAL = 15
local MAX_LIVES = 3
-- The end screens' buttons take the keyboard and gamepad focus only after
-- this long, so the jump that reached the flag doesn't press "Play again"
local END_SCREEN_DELAY = 0.6
-- How the screen goes to another scene
local FADE = { transition = "fade", seconds = 0.5 }
local MUSIC = "assets/sounds/level_music.wav"
local MUSIC_VOLUME = 0.45

local camera, player, sky, hills

function init(scene_id, entity_id)
    set_gravity(0, 1500)
    camera = get_camera(scene_id)
    player = find_entity(scene_id, "player")
    sky = find_entity(scene_id, "sky")
    hills = find_entity(scene_id, "hills")
    local state = script_state.state
    state.lives, state.gems, state.time = MAX_LIVES, 0, 0
    state.won, state.game_over, state.checkpoint = false, false, nil
    state.music_lowered = false
    -- The title's music fades out as this one fades in. Playing again, the
    -- same track plays on, back up from under the end screen
    play_music(MUSIC, { volume = MUSIC_VOLUME, crossfade = 0.8 })
    -- The camera keeps the player in view, a moment behind, and doesn't
    -- show what's beyond the level's ends
    camera_follow(player, {
        smoothing = 0.1,
        bounds = { x = 0, y = 0, width = LEVEL_WIDTH, height = VIEW_HEIGHT },
    })
end

-- The sky moves along with the camera (shakes included), the hills at half
-- its speed. The camera has moved for this frame before updates run.
local function place_backgrounds(scene_id)
    local left = get_attribute(scene_id, camera, "x") - VIEW_WIDTH / 2
    local top = get_attribute(scene_id, camera, "y") - VIEW_HEIGHT / 2
    set_position(scene_id, sky, left, top)
    set_position(scene_id, hills, left * 0.5, HILLS_Y)
end

local function heads_up(state)
    draw_rect(10, 10, 190, 74, { color = "#10182c90", radius = 10 })
    for i = 1, MAX_LIVES do
        draw_image("assets/images/heart.png", 22 + (i - 1) * 32, 18,
            { width = 26, opacity = i <= state.lives and 1 or 0.25 })
    end
    draw_image("assets/images/gem.png", 24, 50, { width = 18 })
    draw_text(state.gems .. " / " .. GEMS_TOTAL, 50, 50, { size = 22 })
    draw_text(string.format("%.1f s", state.time), VIEW_WIDTH - 20, 16,
        { size = 26, align = "right" })
end

local function end_screen(title, color, lines)
    draw_rect(0, 0, VIEW_WIDTH, VIEW_HEIGHT, { color = "#00000050" })
    draw_rect(290, 110, 380, 310, { color = "#10182ce8", radius = 16, outline = "#ffffff30" })
    draw_text(title, VIEW_WIDTH / 2, 132, { size = 46, color = color, align = "center" })
    for i, line in ipairs(lines) do
        draw_text(line, VIEW_WIDTH / 2, 200 + (i - 1) * 34, { size = 24, align = "center" })
    end
end

-- The end screens' two buttons: again, focused by default, or the title
-- screen (also the "back" action: Escape or gamepad B). The arrow keys or
-- the d-pad move the focus, and Enter or gamepad A press the focused button.
local function end_buttons(state, again_label)
    local ready = accumulated_time - state.ended_at >= END_SCREEN_DELAY
    if button(again_label, 320, 340, 150, 54, { default = ready }) then
        change_scene("level", FADE)
    elseif button("Title", 490, 340, 150, 54) or (ready and is_action_just_pressed("back")) then
        change_scene("title", FADE)
    end
end

function update(scene_id, entity_id)
    local state = script_state.state
    place_backgrounds(scene_id)
    if not (state.won or state.game_over) then
        state.time = state.time + delta_time
    end
    heads_up(state)

    -- Under the end screens, the music plays on, quieter
    if (state.won or state.game_over) and not state.music_lowered then
        play_music(MUSIC, { volume = 0.12, crossfade = 0.6 })
        state.music_lowered = true
    end

    if state.won then
        local time = string.format("Time: %.1f s", state.time)
        if state.new_best_time then
            time = time .. "  (best!)"
        end
        end_screen("You made it!", "#ffd84a", { "Gems: " .. state.gems .. " / " .. GEMS_TOTAL, time })
        end_buttons(state, "Play again")
    elseif state.game_over then
        end_screen("Out of hearts", "#ff8080", { "Gems: " .. state.gems .. " / " .. GEMS_TOTAL })
        end_buttons(state, "Try again")
    elseif is_action_just_pressed("menu") then
        change_scene("title", FADE)
    end
end
