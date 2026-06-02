#![no_main]
use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;
use engine_physics::broadphase::{SpatialHashBroadphase, BroadphaseEntry};
use engine_math::Vec3;

#[derive(Arbitrary, Debug)]
enum BroadphaseOp {
    Insert {
        center: [f32; 3],
        half: [f32; 3],
        layers: u32,
        mask: u32,
    },
    SetCellSize {
        size: f32,
    },
    Clear,
}

fn sanitize_f32(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

fn sanitize_vec3(v: [f32; 3]) -> Vec3 {
    Vec3::new(sanitize_f32(v[0]), sanitize_f32(v[1]), sanitize_f32(v[2]))
}

fuzz_target!(|ops: Vec<BroadphaseOp>| {
    let mut bp = SpatialHashBroadphase::new(2.0);
    let mut entity_counter: u32 = 0;

    for op in ops {
        match op {
            BroadphaseOp::Insert { center, half, layers, mask } => {
                let half = sanitize_vec3(half);
                // Ensure positive half-extents
                let half = Vec3::new(
                    half.x.abs().max(0.01),
                    half.y.abs().max(0.01),
                    half.z.abs().max(0.01),
                );
                // Cap center to prevent extreme spatial positions
                let center = sanitize_vec3(center);
                let center = Vec3::new(
                    center.x.clamp(-1000.0, 1000.0),
                    center.y.clamp(-1000.0, 1000.0),
                    center.z.clamp(-1000.0, 1000.0),
                );

                bp.insert(BroadphaseEntry {
                    entity_index: entity_counter,
                    center,
                    half_extents: half,
                    collision_layers: layers,
                    collision_mask: mask,
                });
                entity_counter += 1;
            }
            BroadphaseOp::SetCellSize { size } => {
                let size = sanitize_f32(size).abs().max(0.1);
                bp.set_cell_size(size);
            }
            BroadphaseOp::Clear => {
                bp.clear();
                entity_counter = 0;
            }
        }
    }

    // Compute pairs and verify invariants
    let pairs = bp.compute_pairs();

    // Verify: no duplicate pairs
    for i in 0..pairs.len() {
        for j in (i + 1)..pairs.len() {
            assert!(
                !(pairs[i].index_a == pairs[j].index_a && pairs[i].index_b == pairs[j].index_b),
                "Duplicate pair found: ({}, {})",
                pairs[i].index_a,
                pairs[i].index_b,
            );
        }
    }

    // Verify: all pairs have a < b (canonical ordering)
    for pair in &pairs {
        assert!(
            pair.index_a < pair.index_b,
            "Pair not canonical: ({}, {})",
            pair.index_a,
            pair.index_b,
        );
    }

    // Verify: entry_count is consistent
    assert_eq!(bp.entry_count(), entity_counter as usize);
});
