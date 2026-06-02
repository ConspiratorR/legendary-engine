-- health_system.lua
-- Demonstrates component creation and modification from Lua.

local spawn_timer = 0
local SPAWN_INTERVAL = 2.0

function update(dt)
    -- Spawn a new entity every SPAWN_INTERVAL seconds
    spawn_timer = spawn_timer + dt
    if spawn_timer >= SPAWN_INTERVAL then
        spawn_timer = spawn_timer - SPAWN_INTERVAL
        local entity = world:spawn()
        world:add(entity, "Health", 100.0)
        world:add(entity, "Position", { x = 0.0, y = 0.0, z = 0.0 })
        print("Spawned entity " .. entity .. " with 100 HP")
    end

    -- Process all entities with Health
    local entities = world:entities("Health")
    for _, entity in ipairs(entities) do
        local hp = world:get(entity, "Health")

        -- Slowly regenerate health
        if hp < 100.0 then
            local new_hp = math.min(hp + 5.0 * dt, 100.0)
            world:set(entity, "Health", new_hp)
        end

        -- Despawn entities with zero or negative health
        if hp <= 0.0 then
            print("Entity " .. entity .. " died!")
            world:despawn(entity)
        end
    end
end
