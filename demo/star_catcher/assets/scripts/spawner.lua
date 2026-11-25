-- Runs a round: drops stars (catch them) and rocks (avoid them) from the top
-- of the field, faster and faster, and shows the score. After a rock hits
-- the paddle, Space plays again and Esc returns to the title screen.
--
-- Stars and rocks are copies of the templates in the "prefabs" scene.
--
-- Tuning attributes on this entity:
--   spawn_interval (Float): seconds between drops at the start
--   rock_chance (Float): 0..1, share of drops that are rocks
--   seed (Integer): random seed; 0 picks a different game every time

local FIELD_WIDTH = 480

-- Runs every time the scene starts, including after "play again"
function init(scene_id, entity_id)
    set_gravity(0, 260)
    local seed = get_attribute(scene_id, entity_id, "seed") or 0
    if seed ~= 0 then
        math.randomseed(seed)
    end
    local state = script_state.state
    state.score = 0
    state.game_over = false
    state.round_start = accumulated_time
    state.next_drop = accumulated_time + 1.0
end

local function drop(scene_id, kind)
    local x = math.random(20, FIELD_WIDTH - 48)
    local id = spawn(scene_id, "prefabs/" .. kind, x, -40)
    -- A little spin; stars tumble as they fall
    set_angular_velocity(id, (math.random() - 0.5) * 6)
end

local function game_over_screen(scene_id)
    draw_text("Game over", 240, 250, {size = 56, color = "#ff6060", align = "center"})
    draw_text("Space or (A): play again", 240, 330, {size = 26, align = "center"})
    draw_text("Esc or (B): title screen", 240, 366, {size = 26, align = "center"})
    if is_action_just_pressed("start") then
        -- Changing to the current scene starts it over
        change_scene(scene_id, {transition = "wipe_down", seconds = 0.4})
    elseif is_action_just_pressed("back") then
        change_scene("title", {transition = "fade", seconds = 0.4})
    end
end

function update(scene_id, entity_id)
    local state = script_state.state
    -- Text is drawn for one frame, so every frame
    draw_text("Stars: " .. state.score, 12, 8, {size = 28, color = "#ffd84a"})
    if state.game_over then
        game_over_screen(scene_id)
        return
    end

    if accumulated_time < state.next_drop then
        return
    end

    -- Drops come faster over two minutes, down to a third of the interval
    local elapsed = accumulated_time - state.round_start
    local interval = get_attribute(scene_id, entity_id, "spawn_interval") or 1.0
    interval = interval * math.max(0.35, 1 - elapsed / 120)
    state.next_drop = accumulated_time + interval

    local rock_chance = get_attribute(scene_id, entity_id, "rock_chance") or 0.2
    if math.random() < rock_chance then
        drop(scene_id, "rock")
    else
        drop(scene_id, "star")
    end
end
