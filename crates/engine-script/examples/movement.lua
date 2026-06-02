-- movement.lua
-- Demonstrates a simple movement system in Lua.
-- Requires Position and Velocity components to be registered.

function update(dt)
    -- Get all entities with Position and Velocity
    local positions = world:entities("Position")
    for _, entity in ipairs(positions) do
        if world:has(entity, "Velocity") then
            local pos = world:get(entity, "Position")
            local vel = world:get(entity, "Velocity")

            -- Update position based on velocity
            world:set(entity, "Position", {
                x = pos.x + vel.x * dt,
                y = pos.y + vel.y * dt,
                z = pos.z + vel.z * dt,
            })
        end
    end
end
