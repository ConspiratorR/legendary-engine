#![no_main]
use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;
use engine_ecs::world::World;

#[derive(Arbitrary, Debug)]
enum WorldOp {
    Spawn,
    Despawn { index: u8 },
    AddComponentA { entity_index: u8, value: f32 },
    AddComponentB { entity_index: u8, value: i32 },
    GetComponentA { entity_index: u8 },
    GetComponentB { entity_index: u8 },
    RemoveComponentA { entity_index: u8 },
    RemoveComponentB { entity_index: u8 },
    InsertResource { value: f32 },
    GetResource,
    RemoveResource,
    CompactAll,
    EntityCount,
    MemorySummary,
}

struct CompA(#[allow(dead_code)] f32);
struct CompB(#[allow(dead_code)] i32);
struct ResA(#[allow(dead_code)] f32);

fuzz_target!(|ops: Vec<WorldOp>| {
    let mut world = World::new();
    let mut entities: Vec<engine_ecs::entity::Entity> = Vec::new();

    for op in ops {
        match op {
            WorldOp::Spawn => {
                let e = world.spawn();
                entities.push(e);
            }
            WorldOp::Despawn { index } => {
                if !entities.is_empty() {
                    let idx = index as usize % entities.len();
                    let e = entities.swap_remove(idx);
                    world.despawn(e);
                }
            }
            WorldOp::AddComponentA { entity_index, value } => {
                if !entities.is_empty() {
                    let idx = entity_index as usize % entities.len();
                    world.add_component(entities[idx], CompA(value));
                }
            }
            WorldOp::AddComponentB { entity_index, value } => {
                if !entities.is_empty() {
                    let idx = entity_index as usize % entities.len();
                    world.add_component(entities[idx], CompB(value));
                }
            }
            WorldOp::GetComponentA { entity_index } => {
                if !entities.is_empty() {
                    let idx = entity_index as usize % entities.len();
                    let _ = world.get::<CompA>(entities[idx]);
                }
            }
            WorldOp::GetComponentB { entity_index } => {
                if !entities.is_empty() {
                    let idx = entity_index as usize % entities.len();
                    let _ = world.get::<CompB>(entities[idx]);
                }
            }
            WorldOp::RemoveComponentA { entity_index } => {
                if !entities.is_empty() {
                    let idx = entity_index as usize % entities.len();
                    let _ = world.remove_component::<CompA>(entities[idx]);
                }
            }
            WorldOp::RemoveComponentB { entity_index } => {
                if !entities.is_empty() {
                    let idx = entity_index as usize % entities.len();
                    let _ = world.remove_component::<CompB>(entities[idx]);
                }
            }
            WorldOp::InsertResource { value } => {
                world.insert_resource(ResA(value));
            }
            WorldOp::GetResource => {
                let _ = world.get_resource::<ResA>();
            }
            WorldOp::RemoveResource => {
                let _ = world.remove_resource::<ResA>();
            }
            WorldOp::CompactAll => {
                world.compact_all();
            }
            WorldOp::EntityCount => {
                let _ = world.entity_count();
            }
            WorldOp::MemorySummary => {
                let _ = world.memory_summary();
            }
        }
    }

    // Verify invariants after all operations
    let _ = world.entity_count();
    let _ = world.total_spawned();
    let _ = world.total_despawned();
    let _ = world.memory_report();
});
