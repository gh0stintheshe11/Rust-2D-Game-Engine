-- The player: runs and jumps with the keyboard or a gamepad, drops through
-- wooden planks with down, breaks bricks by jumping into them from below,
-- and loses a heart in a pit or on spikes.
--
-- Tuning (attributes, edit in the Inspector):
--   speed (Float): running speed, px/s
--   jump_speed (Float): take-off speed, px/s (with the level's gravity of
--     1500 px/s², 600 jumps 120 px high)

local WIDTH, HEIGHT = 28, 42 -- the sprite's frame, which is also its body
local COYOTE_TIME = 0.1 -- a jump still works this long after leaving a ledge
local JUMP_BUFFER = 0.12 -- a jump pressed this early before landing happens on landing
local SHORT_HOP_SPEED = 220 -- letting go of jump while rising faster cuts the jump short
local FALL_LIMIT = 620 -- below this, the player fell into a pit
local INVULNERABLE_TIME = 1.5 -- after losing a heart
local BRICK = 3 -- the tileset's brick tile

local start = { x = 0, y = 0 }
local ground, spikes, dust, debris
local since_ground, since_jump_pressed, was_grounded, last_vy, invulnerable

function init(scene_id, entity_id)
    start.x = get_attribute(scene_id, entity_id, "x")
    start.y = get_attribute(scene_id, entity_id, "y")
    ground = find_entity(scene_id, "ground")
    spikes = find_entity(scene_id, "spikes")
    dust = find_entity(scene_id, "dust")
    debris = find_entity(scene_id, "debris")
    since_ground, since_jump_pressed = 1, 1
    was_grounded, last_vy, invulnerable = false, 0, 0
end

-- The project's input actions (Project > Input Actions) say which keys
-- and gamepad controls these are. The direction to run goes from -1 to 1:
-- the stick runs slower when pushed part way.
local function read_input()
    local move = get_action_axis("left", "right")
    return move, is_action_just_pressed("jump"), is_action_pressed("jump")
end

-- Standing on something: two short rays down from the feet (rays skip
-- sensors, like gems and spikes, and the player itself)
local function on_ground(entity_id, x, y)
    for _, foot_x in ipairs({ x + 3, x + WIDTH - 3 }) do
        if raycast(foot_x, y + HEIGHT - 2, foot_x, y + HEIGHT + 3, entity_id) then
            return true
        end
    end
    return false
end

local function dust_at(scene_id, x, y, count)
    emit_particles(scene_id, dust, count, x, y)
end

local function respawn(scene_id, entity_id)
    local spot = script_state.state.checkpoint or start
    set_position(scene_id, entity_id, spot.x, spot.y)
    set_velocity(entity_id, 0, 0)
    invulnerable = INVULNERABLE_TIME
end

-- Lose a heart and start over at the last checkpoint. While invulnerable
-- (just after), nothing hurts, but a fall still brings the player back.
local function hurt(scene_id, entity_id, fell)
    local state = script_state.state
    if invulnerable > 0 then
        if fell then
            respawn(scene_id, entity_id)
        end
        return
    end
    state.lives = state.lives - 1
    play_sound("assets/sounds/hurt.wav", { volume = 0.8 })
    camera_shake(7, 0.35)
    respawn(scene_id, entity_id)
    if state.lives <= 0 then
        state.game_over = true
        state.ended_at = accumulated_time
    end
end

-- Break the bricks the head is about to hit
local function break_bricks(scene_id, x, y, vy)
    local head = y - 2 + vy * delta_time
    local broke = false
    for _, head_x in ipairs({ x + 4, x + WIDTH - 4 }) do
        local column, row = get_tile_cell(scene_id, ground, head_x, head)
        if get_tile(scene_id, ground, column, row) == BRICK then
            set_tile(scene_id, ground, column, row, nil)
            emit_particles(scene_id, debris, 12, column * 32 + 16, row * 32 + 16)
            broke = true
        end
    end
    if broke then
        play_sound("assets/sounds/brick.wav", { volume = 0.8 })
        camera_shake(3, 0.15)
    end
    return broke
end

function update(scene_id, entity_id)
    local state = script_state.state
    local x = get_attribute(scene_id, entity_id, "x")
    local y = get_attribute(scene_id, entity_id, "y")
    local vx, vy = get_velocity(entity_id)

    -- The end screens: stand still
    if state.won or state.game_over then
        set_velocity(entity_id, 0, vy)
        play_animation(scene_id, entity_id, "0,0,0,0,1", { fps = 3 })
        set_attribute(scene_id, entity_id, "opacity", 1.0)
        return
    end

    if y > FALL_LIMIT then
        hurt(scene_id, entity_id, true)
        return
    end
    invulnerable = math.max(0, invulnerable - delta_time)
    for _, other in ipairs(get_colliding_entities(entity_id)) do
        if other == spikes then
            hurt(scene_id, entity_id, false)
            return
        end
    end

    local move, jump, held = read_input()
    local grounded = vy > -1 and on_ground(entity_id, x, y)
    -- On a plank, down drops through it (on solid ground, nothing happens).
    -- The stick has to be pushed well down: running down a slope, it's
    -- pushed diagonally (the action's threshold).
    if is_action_just_pressed("down") and grounded then
        drop_through_platforms(entity_id)
    end
    since_ground = grounded and 0 or since_ground + delta_time
    since_jump_pressed = jump and 0 or since_jump_pressed + delta_time

    if since_jump_pressed <= JUMP_BUFFER and since_ground <= COYOTE_TIME then
        vy = -get_attribute(scene_id, entity_id, "jump_speed")
        since_ground, since_jump_pressed = COYOTE_TIME + 1, JUMP_BUFFER + 1
        grounded = false
        play_sound("assets/sounds/jump.wav", { volume = 0.5 })
        dust_at(scene_id, x + WIDTH / 2, y + HEIGHT, 6)
    elseif not held and vy < -SHORT_HOP_SPEED then
        vy = -SHORT_HOP_SPEED
    end
    if vy < 0 and break_bricks(scene_id, x, y, vy) then
        vy = 60
    end

    if grounded and not was_grounded and last_vy > 350 then
        dust_at(scene_id, x + WIDTH / 2, y + HEIGHT, 8)
    end
    was_grounded, last_vy = grounded, vy

    set_velocity(entity_id, move * get_attribute(scene_id, entity_id, "speed"), vy)

    -- Looks
    if move < 0 then
        set_attribute(scene_id, entity_id, "flip_x", true)
    elseif move > 0 then
        set_attribute(scene_id, entity_id, "flip_x", false)
    end
    if not grounded then
        play_animation(scene_id, entity_id, vy < 0 and "6" or "7")
    elseif math.abs(move) > 0.1 then
        play_animation(scene_id, entity_id, "2-5", { fps = 12 })
    else
        play_animation(scene_id, entity_id, "0,0,0,0,1", { fps = 3 })
    end
    local blink = invulnerable > 0 and math.floor(invulnerable * 10) % 2 == 0
    set_attribute(scene_id, entity_id, "opacity", blink and 0.3 or 1.0)
end
