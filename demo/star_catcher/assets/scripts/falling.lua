-- Attached to every falling star and rock by the spawner.

local TALLY_PER_ROW = 30

-- A small star at the top of the screen for every star caught
local function add_tally(scene_id, score)
    local tally = add_entity(scene_id, "tally")
    add_image(tally, "assets/images/tally.png")
    local column = (score - 1) % TALLY_PER_ROW
    local row = math.floor((score - 1) / TALLY_PER_ROW)
    set_position(scene_id, tally, 8 + column * 15, 8 + row * 15)
    set_z(scene_id, tally, 5)
end

function on_collision(scene_id, entity_id, other_id)
    local kind = get_entity_name(scene_id, entity_id)
    local other = get_entity_name(scene_id, other_id)

    if other == "paddle" then
        if kind == "star" then
            play_sound("assets/sounds/catch.wav")
            local state = script_state.state
            state.score = state.score + 1
            add_tally(scene_id, state.score)
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
