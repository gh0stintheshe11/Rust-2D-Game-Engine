-- The player's paddle: move with the arrow keys or A / D.
--
-- Tuning (edit in the Inspector, no code changes needed):
--   speed (Float): pixels per second

local FIELD_WIDTH = 480

function update(scene_id, entity_id)
    local speed = get_attribute(scene_id, entity_id, "speed") or 320
    local direction = 0
    if is_key_pressed("ArrowLeft") or is_key_pressed("A") then
        direction = direction - 1
    end
    if is_key_pressed("ArrowRight") or is_key_pressed("D") then
        direction = direction + 1
    end

    -- Stay on the field
    local x = get_attribute(scene_id, entity_id, "x")
    local width = get_attribute(scene_id, entity_id, "collider_width") or 96
    if (x <= 0 and direction < 0) or (x >= FIELD_WIDTH - width and direction > 0) then
        direction = 0
    end

    -- The paddle is kinematic: it moves only by velocity, and falling
    -- objects can't push it around
    set_velocity(entity_id, direction * speed, 0)
end
