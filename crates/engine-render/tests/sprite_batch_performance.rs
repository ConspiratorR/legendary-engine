// crates/engine-render/tests/sprite_batch_performance.rs

use engine_math::{Mat4, Vec2, Vec3};
use engine_render::sprite::{SpriteBatch, SpriteDraw, collect_batches};

#[test]
fn test_sprite_batch_instance_data() {
    let mut batch = SpriteBatch::new(0);

    let draw = SpriteDraw {
        world_matrix: Mat4::from_translation(Vec3::new(100.0, 200.0, 0.0)),
        color: [1.0, 1.0, 1.0, 1.0],
        size: Vec2::new(50.0, 50.0),
        texture_id: 0,
        flip_x: false,
        flip_y: false,
        depth: 0.0,
        uv_region: [0.0, 0.0, 1.0, 1.0],
    };

    batch.push(&draw);
    batch.push(&draw);
    batch.push(&draw);

    assert_eq!(batch.instance_data.len(), 3);
    assert_eq!(batch.vertices.len(), 12); // 3 sprites * 4 vertices
    assert_eq!(batch.indices.len(), 18); // 3 sprites * 6 indices

    batch.update_indirect_cmd();
    assert_eq!(batch.indirect_cmd.instance_count, 3);
    assert_eq!(batch.indirect_cmd.index_count, 18);
}

#[test]
fn test_collect_batches_with_instances() {
    let draws = vec![
        SpriteDraw {
            texture_id: 1,
            world_matrix: Mat4::IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(100.0, 100.0),
            flip_x: false,
            flip_y: false,
            depth: 0.0,
            uv_region: [0.0, 0.0, 1.0, 1.0],
        },
        SpriteDraw {
            texture_id: 0,
            world_matrix: Mat4::IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(100.0, 100.0),
            flip_x: false,
            flip_y: false,
            depth: 0.0,
            uv_region: [0.0, 0.0, 1.0, 1.0],
        },
        SpriteDraw {
            texture_id: 1,
            world_matrix: Mat4::IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(100.0, 100.0),
            flip_x: false,
            flip_y: false,
            depth: 0.0,
            uv_region: [0.0, 0.0, 1.0, 1.0],
        },
    ];

    let batches = collect_batches(&draws);
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].texture_id, 1);
    assert_eq!(batches[0].instance_data.len(), 2);
    assert_eq!(batches[1].texture_id, 0);
    assert_eq!(batches[1].instance_data.len(), 1);
}
