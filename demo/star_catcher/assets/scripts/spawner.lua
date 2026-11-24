-- Drops stars (catch them) and rocks (avoid them) from the top of the field,
-- faster and faster.
--
-- Tuning attributes on this entity:
--   spawn_interval (Float): seconds between drops at the start
--   rock_chance (Float): 0..1, share of drops that are rocks
--   seed (Integer): random seed; 0 picks a different game every time

local FIELD_WIDTH = 480

function init(scene_id, entity_id)
    set_gravity(0, 260)
    local seed = get_attribute(scene_id, entity_id, "seed") or 0
    if seed ~= 0 then
        math.randomseed(seed)
    end
    script_state.state.score = 0
    script_state.state.next_drop = 1.0
end

local function drop(scene_id, kind)
    local x = math.random(20, FIELD_WIDTH - 48)
    local id = create_physical_entity(scene_id, kind, x, -40, 1)
    add_image(id, "assets/images/" .. kind .. ".png")
    create_attribute_string(scene_id, id, "collider_shape", "circle")
    set_script(id, "assets/scripts/falling.lua")
    add_entity_to_physics_engine(id)
    -- A little spin; stars tumble as they fall
    set_angular_velocity(id, (math.random() - 0.5) * 6)
end

function update(scene_id, entity_id)
    local state = script_state.state
    if accumulated_time < state.next_drop then
        return
    end

    -- Drops come faster over two minutes, down to a third of the interval
    local interval = get_attribute(scene_id, entity_id, "spawn_interval") or 1.0
    interval = interval * math.max(0.35, 1 - accumulated_time / 120)
    state.next_drop = accumulated_time + interval

    local rock_chance = get_attribute(scene_id, entity_id, "rock_chance") or 0.2
    if math.random() < rock_chance then
        drop(scene_id, "rock")
    else
        drop(scene_id, "star")
    end
end
