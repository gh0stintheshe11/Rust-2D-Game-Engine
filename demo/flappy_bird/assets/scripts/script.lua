-- Flappy Bird player script.
--
-- Tuning (edit these in the Inspector on the bird entity - no code needed):
-- - `gravity_scale` (Float): how fast the bird falls
--   (global gravity is 50 px/s^2; the bird uses 18x = 900 px/s^2)
-- - `jump_velocity` (Float): flap strength, negative = up

function update(scene_id, entity_id)
    if is_key_just_pressed("Space") then
        local jump = get_attribute(scene_id, entity_id, "jump_velocity") or -260.0
        set_velocity(entity_id, 0.0, jump)
    end

    -- Game over when the bird touches a pipe or the ground
    local colliding = get_colliding_entities(entity_id)
    for i = 1, #colliding do
        local name = get_entity_name(scene_id, colliding[i])
        if name ~= nil then
            if string.sub(name, 1, 8) == "top_pipe"
                or string.sub(name, 1, 11) == "bottom_pipe"
                or string.sub(name, 1, 6) == "ground" then
                end_game()
                return
            end
        end
    end
end
