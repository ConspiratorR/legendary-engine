use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use engine_math::Mat4;
use engine_render::graph::RenderGraph;
use engine_render::sprite::{SpriteBatch, SpriteDraw};

fn bench_render_graph_creation(c: &mut Criterion) {
    c.bench_function("render_graph_creation", |b| {
        b.iter(RenderGraph::new);
    });
}

fn bench_render_graph_reset(c: &mut Criterion) {
    c.bench_function("render_graph_reset", |b| {
        let mut graph = RenderGraph::new();
        b.iter(|| {
            graph.reset();
        });
    });
}

fn bench_sprite_batch_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("sprite_batch_creation");

    for count in [100, 500, 1000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let mut batch = SpriteBatch::new(0);
                for i in 0..count {
                    let draw = SpriteDraw {
                        world_matrix: Mat4::from_translation([i as f32 * 10.0, 0.0, 0.0].into()),
                        color: [1.0, 1.0, 1.0, 1.0],
                        size: [64.0, 64.0].into(),
                        texture_id: 0,
                        flip_x: false,
                        flip_y: false,
                        depth: 0.0,
                        uv_region: [0.0, 0.0, 1.0, 1.0],
                    };
                    batch.push(&draw);
                }
                batch
            });
        });
    }

    group.finish();
}

fn bench_sprite_batch_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("sprite_batch_update");

    for count in [100, 500, 1000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let mut batch = SpriteBatch::new(0);
            for i in 0..count {
                let draw = SpriteDraw {
                    world_matrix: Mat4::from_translation([i as f32 * 10.0, 0.0, 0.0].into()),
                    color: [1.0, 1.0, 1.0, 1.0],
                    size: [64.0, 64.0].into(),
                    texture_id: 0,
                    flip_x: false,
                    flip_y: false,
                    depth: 0.0,
                    uv_region: [0.0, 0.0, 1.0, 1.0],
                };
                batch.push(&draw);
            }

            b.iter(|| {
                batch.update_indirect_cmd();
            });
        });
    }

    group.finish();
}

fn bench_collect_batches(c: &mut Criterion) {
    let mut group = c.benchmark_group("collect_batches");

    for count in [100, 500, 1000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let mut draws = Vec::new();
            for i in 0..count {
                draws.push(SpriteDraw {
                    world_matrix: Mat4::from_translation([i as f32 * 10.0, 0.0, 0.0].into()),
                    color: [1.0, 1.0, 1.0, 1.0],
                    size: [64.0, 64.0].into(),
                    texture_id: (i % 3) as u64, // Multiple textures
                    flip_x: false,
                    flip_y: false,
                    depth: 0.0,
                    uv_region: [0.0, 0.0, 1.0, 1.0],
                });
            }

            b.iter(|| {
                engine_render::sprite::collect_batches(&draws);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_render_graph_creation,
    bench_render_graph_reset,
    bench_sprite_batch_creation,
    bench_sprite_batch_update,
    bench_collect_batches,
);
criterion_main!(benches);
