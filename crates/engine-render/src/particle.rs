//! 2D particle system (CPU mode).
//!
//! Provides particle emitters, per-particle state, piecewise-linear curves
//! for animating properties over a particle's lifetime, and an ECS update
//! system that advances simulation and produces [`SpriteDraw`] entries for
//! the existing sprite rendering path.

use engine_asset::asset::Handle;
use engine_asset::types::Texture;
use engine_ecs::world::World;
use engine_math::{Mat4, Vec2, Vec3};
use std::collections::HashMap;
use std::ops::Range;

use crate::sprite::SpriteDraw;

// ---------------------------------------------------------------------------
// PRNG (xorshift64)
// ---------------------------------------------------------------------------

struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 { 1 } else { seed })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform f32 in [0, 1).
    fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Uniform f32 in [lo, hi).
    fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }
}

// ---------------------------------------------------------------------------
// Curve
// ---------------------------------------------------------------------------

/// Piecewise-linear interpolation curve over [0, 1].
///
/// Points are `(t, value)` pairs with `t` in `[0, 1]`, sorted ascending.
/// Evaluating outside the defined range clamps to the nearest endpoint.
#[derive(Debug, Clone)]
pub struct Curve<T> {
    pub points: Vec<(f32, T)>,
}

impl Curve<f32> {
    pub fn constant(value: f32) -> Self {
        Self {
            points: vec![(0.0, value)],
        }
    }

    pub fn linear(from: f32, to: f32) -> Self {
        Self {
            points: vec![(0.0, from), (1.0, to)],
        }
    }

    pub fn evaluate(&self, t: f32) -> f32 {
        eval_curve(&self.points, t, |a, b, t| a + (b - a) * t)
    }
}

impl Curve<Vec2> {
    pub fn evaluate(&self, t: f32) -> Vec2 {
        eval_curve(&self.points, t, |a: Vec2, b: Vec2, t| a.lerp(b, t))
    }
}

fn eval_curve<T: Copy>(points: &[(f32, T)], t: f32, lerp: impl Fn(T, T, f32) -> T) -> T {
    if points.is_empty() {
        panic!("cannot evaluate empty curve");
    }
    if points.len() == 1 || t <= points[0].0 {
        return points[0].1;
    }
    if t >= points.last().expect("checked: points is non-empty").0 {
        return points.last().expect("checked: points is non-empty").1;
    }
    for window in points.windows(2) {
        let (t0, v0) = window[0];
        let (t1, v1) = window[1];
        if t >= t0 && t <= t1 {
            let frac = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
            return lerp(v0, v1, frac);
        }
    }
    points.last().expect("checked: points is non-empty").1
}

// ---------------------------------------------------------------------------
// Particle
// ---------------------------------------------------------------------------

/// Runtime state of a single particle.
#[derive(Debug, Clone)]
pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    pub lifetime: f32,
    pub age: f32,
    pub size: f32,
    pub color: [f32; 4],
}

// ---------------------------------------------------------------------------
// ParticleEmitter (ECS component)
// ---------------------------------------------------------------------------

/// Configuration for a particle emitter. Attach as an ECS component to
/// an entity, then call [`update_particles`] each frame.
#[derive(Clone)]
pub struct ParticleEmitter {
    pub rate: f32,
    pub burst: Option<u32>,
    pub max_particles: u32,
    pub lifetime: Range<f32>,
    pub speed: Range<f32>,
    pub angle: Range<f32>,
    pub size: Range<f32>,
    pub color: [f32; 4],
    pub size_curve: Option<Curve<f32>>,
    pub opacity_curve: Option<Curve<f32>>,
    pub texture: Handle<Texture>,
    pub active: bool,
    pub depth: f32,
    pub(crate) spawn_accumulator: f32,
}

impl ParticleEmitter {
    pub fn new(rate: f32, texture: Handle<Texture>) -> Self {
        Self {
            rate,
            burst: None,
            max_particles: 1000,
            lifetime: 1.0..3.0,
            speed: 50.0..150.0,
            angle: 0.0..std::f32::consts::TAU,
            size: 4.0..8.0,
            color: [1.0, 1.0, 1.0, 1.0],
            size_curve: None,
            opacity_curve: None,
            texture,
            active: true,
            depth: 0.0,
            spawn_accumulator: 0.0,
        }
    }

    pub fn with_burst(mut self, count: u32) -> Self {
        self.burst = Some(count);
        self
    }

    pub fn with_max_particles(mut self, max: u32) -> Self {
        self.max_particles = max;
        self
    }

    pub fn with_lifetime(mut self, range: Range<f32>) -> Self {
        self.lifetime = range;
        self
    }

    pub fn with_speed(mut self, range: Range<f32>) -> Self {
        self.speed = range;
        self
    }

    pub fn with_angle(mut self, range: Range<f32>) -> Self {
        self.angle = range;
        self
    }

    pub fn with_size(mut self, range: Range<f32>) -> Self {
        self.size = range;
        self
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn with_size_curve(mut self, curve: Curve<f32>) -> Self {
        self.size_curve = Some(curve);
        self
    }

    pub fn with_opacity_curve(mut self, curve: Curve<f32>) -> Self {
        self.opacity_curve = Some(curve);
        self
    }

    pub fn with_depth(mut self, depth: f32) -> Self {
        self.depth = depth;
        self
    }
}

// ---------------------------------------------------------------------------
// ParticleSystem (ECS resource)
// ---------------------------------------------------------------------------

/// Per-emitter runtime state.
struct EmitterRuntime {
    particles: Vec<Particle>,
    burst_handled: bool,
}

/// ECS resource that manages active particle pools for all emitter entities.
/// Insert via `world.insert_resource(ParticleSystem::new())`.
pub struct ParticleSystem {
    emitters: HashMap<u32, EmitterRuntime>,
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            emitters: HashMap::new(),
        }
    }

    /// Returns the number of alive particles for a given entity index.
    pub fn particle_count(&self, entity_index: u32) -> usize {
        self.emitters
            .get(&entity_index)
            .map(|e| e.particles.len())
            .unwrap_or(0)
    }

    /// Total alive particles across all emitters.
    pub fn total_particle_count(&self) -> usize {
        self.emitters.values().map(|e| e.particles.len()).sum()
    }
}

// ---------------------------------------------------------------------------
// ParticleDrawBuffer (ECS resource)
// ---------------------------------------------------------------------------

/// Output of the particle update — pre-built [`SpriteDraw`] entries ready
/// for the sprite rendering pipeline. Read this after each
/// [`update_particles`] call and merge into your sprite draw list.
pub struct ParticleDrawBuffer {
    draws: Vec<SpriteDraw>,
}

impl Default for ParticleDrawBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticleDrawBuffer {
    pub fn new() -> Self {
        Self { draws: Vec::new() }
    }

    pub fn draws(&self) -> &[SpriteDraw] {
        &self.draws
    }

    pub fn clear(&mut self) {
        self.draws.clear();
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn make_rng(seed_base: u32, tick: u64) -> XorShift64 {
    XorShift64::new((seed_base as u64).wrapping_mul(6364136223846793005) ^ tick ^ 0xCAFEBABE)
}

fn spawn_particle(emitter: &ParticleEmitter, rng: &mut XorShift64, _total_time: f32) -> Particle {
    let lifetime = rng
        .range_f32(emitter.lifetime.start, emitter.lifetime.end)
        .max(0.001);
    let angle = rng.range_f32(emitter.angle.start, emitter.angle.end);
    let speed = rng.range_f32(emitter.speed.start, emitter.speed.end);
    let size = rng.range_f32(emitter.size.start, emitter.size.end);

    let velocity = Vec2::new(angle.cos() * speed, angle.sin() * speed);
    let base_color = emitter.color;
    let opacity = if let Some(ref curve) = emitter.opacity_curve {
        curve.evaluate(0.0)
    } else {
        base_color[3]
    };

    Particle {
        position: Vec2::ZERO,
        velocity,
        lifetime,
        age: 0.0,
        size,
        color: [base_color[0], base_color[1], base_color[2], opacity],
    }
}

fn update_and_draw_particles(
    particles: &mut [Particle],
    dt: f32,
    emitter: &ParticleEmitter,
    texture_id: u64,
    emitter_pos: Vec2,
    draws: &mut Vec<SpriteDraw>,
) {
    for p in particles.iter_mut() {
        p.age += dt;
        let t = (p.age / p.lifetime).clamp(0.0, 1.0);
        p.position += p.velocity * dt;

        let mut size = p.size;
        if let Some(ref curve) = emitter.size_curve {
            size *= curve.evaluate(t);
        }

        let mut color = p.color;
        if let Some(ref curve) = emitter.opacity_curve {
            color[3] = curve.evaluate(t);
        }

        let world_pos = emitter_pos + p.position;
        draws.push(SpriteDraw {
            world_matrix: Mat4::from_translation(Vec3::new(
                world_pos.x,
                world_pos.y,
                emitter.depth,
            )),
            color,
            size: Vec2::new(size, size),
            texture_id,
            flip_x: false,
            flip_y: false,
            depth: emitter.depth,
            uv_region: [0.0, 0.0, 1.0, 1.0],
        });
    }
}

// ---------------------------------------------------------------------------
// ECS system
// ---------------------------------------------------------------------------

/// Update all active particle emitters and produce draw entries.
///
/// Expects these ECS resources:
/// - [`ParticleSystem`] — runtime state (create with `new()`)
/// - [`ParticleDrawBuffer`] — output buffer (create with `new()`)
/// - [`crate::animation::AnimationTime`] — frame delta time
///
/// Entities with a [`ParticleEmitter`] component are processed each frame.
/// After the call, read [`ParticleDrawBuffer::draws`] to get the
/// [`SpriteDraw`] entries and merge them into your sprite draw list before
/// rendering.
pub fn update_particles(world: &mut World, bridge: &crate::texture_bridge::TextureBridge) {
    let indices: Vec<u32> = world.component_entities::<ParticleEmitter>();

    let dt = world
        .get_resource::<crate::animation::AnimationTime>()
        .map(|t| t.dt)
        .unwrap_or(1.0 / 60.0);

    // Snapshot emitters (immutable read, no borrow conflict with mutable
    // resource access below).
    let emitter_snapshots: Vec<(u32, ParticleEmitter)> = indices
        .iter()
        .filter_map(|&idx| {
            world
                .get_by_index::<ParticleEmitter>(idx)
                .map(|e| (idx, e.clone()))
        })
        .collect();

    // Get or create resources.
    if world.get_resource::<ParticleSystem>().is_none() {
        world.insert_resource(ParticleSystem::new());
    }
    if world.get_resource::<ParticleDrawBuffer>().is_none() {
        world.insert_resource(ParticleDrawBuffer::new());
    }

    // Clear output buffer.
    if let Some(buf) = world.get_resource_mut::<ParticleDrawBuffer>() {
        buf.draws.clear();
    }

    // Process each emitter.
    let mut all_draws: Vec<SpriteDraw> = Vec::new();
    let mut accumulator_updates: Vec<(u32, f32)> = Vec::new();

    for (entity_idx, emitter) in &emitter_snapshots {
        if !emitter.active {
            continue;
        }

        let texture_id = bridge.resolve(&emitter.texture);

        let system = world.get_resource_mut::<ParticleSystem>().unwrap();
        let runtime = system
            .emitters
            .entry(*entity_idx)
            .or_insert_with(|| EmitterRuntime {
                particles: Vec::new(),
                burst_handled: false,
            });

        // Handle burst (once).
        if let Some(burst_count) = emitter.burst
            && !runtime.burst_handled
        {
            let count = burst_count.min(
                emitter
                    .max_particles
                    .saturating_sub(runtime.particles.len() as u32),
            );
            let mut rng = make_rng(*entity_idx, runtime.particles.len() as u64);
            for _ in 0..count {
                runtime
                    .particles
                    .push(spawn_particle(emitter, &mut rng, 0.0));
            }
            runtime.burst_handled = true;
        }

        // Rate-based emission.
        let mut new_accumulator = emitter.spawn_accumulator;
        if emitter.rate > 0.0 {
            new_accumulator += dt * emitter.rate;
            let to_spawn = new_accumulator.floor() as u32;
            new_accumulator -= to_spawn as f32;

            let capacity = emitter
                .max_particles
                .saturating_sub(runtime.particles.len() as u32);
            let count = to_spawn.min(capacity);
            let mut rng = make_rng(*entity_idx, runtime.particles.len() as u64);
            for _ in 0..count {
                runtime
                    .particles
                    .push(spawn_particle(emitter, &mut rng, 0.0));
            }
        }

        accumulator_updates.push((*entity_idx, new_accumulator));

        // Update particles, remove dead, build SpriteDraws.
        runtime.particles.retain(|p| p.age + dt < p.lifetime);

        update_and_draw_particles(
            &mut runtime.particles,
            dt,
            emitter,
            texture_id,
            Vec2::ZERO,
            &mut all_draws,
        );
    }

    // Write back spawn accumulators (no borrow conflict outside the loop).
    for (entity_idx, acc) in accumulator_updates {
        if let Some(e) = world.get_by_index_mut::<ParticleEmitter>(entity_idx) {
            e.spawn_accumulator = acc;
        }
    }

    // Store output.
    if let Some(buf) = world.get_resource_mut::<ParticleDrawBuffer>() {
        buf.draws = all_draws;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::AnimationTime;
    use crate::texture_bridge::TextureBridge;

    fn test_texture() -> Handle<Texture> {
        Handle::new(Texture {
            id: "test_particle".into(),
            width: 1,
            height: 1,
            data: vec![255, 255, 255, 255],
            channels: 4,
            asset_path: std::path::PathBuf::new(),
        })
    }

    // -- Curve tests --

    #[test]
    fn test_curve_constant() {
        let c = Curve::<f32>::constant(5.0);
        assert!((c.evaluate(0.0) - 5.0).abs() < 1e-6);
        assert!((c.evaluate(0.5) - 5.0).abs() < 1e-6);
        assert!((c.evaluate(1.0) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_curve_linear() {
        let c = Curve::<f32>::linear(0.0, 10.0);
        assert!((c.evaluate(0.0) - 0.0).abs() < 1e-6);
        assert!((c.evaluate(0.5) - 5.0).abs() < 1e-6);
        assert!((c.evaluate(1.0) - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_curve_clamp() {
        let c = Curve::<f32>::linear(0.0, 10.0);
        assert!((c.evaluate(-1.0) - 0.0).abs() < 1e-6);
        assert!((c.evaluate(2.0) - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_curve_multi_point() {
        let c = Curve::<f32> {
            points: vec![(0.0, 0.0), (0.5, 10.0), (1.0, 0.0)],
        };
        assert!((c.evaluate(0.0) - 0.0).abs() < 1e-6);
        assert!((c.evaluate(0.25) - 5.0).abs() < 1e-6);
        assert!((c.evaluate(0.5) - 10.0).abs() < 1e-6);
        assert!((c.evaluate(0.75) - 5.0).abs() < 1e-6);
        assert!((c.evaluate(1.0) - 0.0).abs() < 1e-6);
    }

    // -- PRNG tests --

    #[test]
    fn test_prng_range() {
        let mut rng = XorShift64::new(42);
        for _ in 0..1000 {
            let v = rng.range_f32(2.0, 5.0);
            assert!(v >= 2.0 && v < 5.0, "value {} out of range", v);
        }
    }

    #[test]
    fn test_prng_different_seeds() {
        let mut r1 = XorShift64::new(1);
        let mut r2 = XorShift64::new(99);
        // Generate several values; very unlikely all match.
        let mut all_equal = true;
        for _ in 0..10 {
            if (r1.next_f32() - r2.next_f32()).abs() > 1e-6 {
                all_equal = false;
                break;
            }
        }
        assert!(
            !all_equal,
            "different seeds should produce different sequences"
        );
    }

    // -- ParticleEmitter tests --

    #[test]
    fn test_emitter_builder() {
        let tex = test_texture();
        let emitter = ParticleEmitter::new(100.0, tex)
            .with_burst(50)
            .with_max_particles(500)
            .with_lifetime(0.5..2.0)
            .with_speed(10.0..50.0)
            .with_size(2.0..6.0)
            .with_color([1.0, 0.0, 0.0, 1.0])
            .with_depth(5.0);

        assert_eq!(emitter.rate, 100.0);
        assert_eq!(emitter.burst, Some(50));
        assert_eq!(emitter.max_particles, 500);
        assert_eq!(emitter.lifetime, 0.5..2.0);
        assert_eq!(emitter.depth, 5.0);
    }

    // -- ParticleSystem tests --

    #[test]
    fn test_particle_system_counts() {
        let mut system = ParticleSystem::new();
        assert_eq!(system.total_particle_count(), 0);
        assert_eq!(system.particle_count(0), 0);

        system.emitters.insert(
            0,
            EmitterRuntime {
                particles: vec![
                    Particle {
                        position: Vec2::ZERO,
                        velocity: Vec2::ZERO,
                        lifetime: 1.0,
                        age: 0.0,
                        size: 4.0,
                        color: [1.0; 4],
                    };
                    3
                ],
                burst_handled: false,
            },
        );
        assert_eq!(system.particle_count(0), 3);
        assert_eq!(system.total_particle_count(), 3);
    }

    // -- Integration test --

    #[test]
    fn test_update_particles_integration() {
        let mut world = World::new();

        let tex = test_texture();
        let emitter = ParticleEmitter::new(10.0, tex)
            .with_max_particles(100)
            .with_lifetime(1.0..1.0)
            .with_speed(0.0..0.0)
            .with_size(4.0..4.0)
            .with_angle(0.0..0.0);

        let e = world.spawn();
        world.add_component(e, emitter);
        world.insert_resource(ParticleSystem::new());
        world.insert_resource(ParticleDrawBuffer::new());
        world.insert_resource(AnimationTime { dt: 0.5 });

        // Need a TextureBridge for resolve().
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .unwrap();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("test_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bridge = TextureBridge::new(&device, &queue, layout);

        update_particles(&mut world, &bridge);

        let buf = world.get_resource::<ParticleDrawBuffer>().unwrap();
        // At 10 rate and 0.5s dt, should have spawned ~5 particles.
        assert!(
            buf.draws.len() >= 4 && buf.draws.len() <= 6,
            "expected 4-6 draws, got {}",
            buf.draws.len()
        );

        // All particles should have size 4.0 and depth 0.0.
        for draw in &buf.draws {
            assert!((draw.size.x - 4.0).abs() < 1e-4);
            assert!((draw.size.y - 4.0).abs() < 1e-4);
            assert!((draw.depth - 0.0).abs() < 1e-4);
        }

        // System should track particles.
        let sys = world.get_resource::<ParticleSystem>().unwrap();
        assert_eq!(sys.particle_count(e.index()), buf.draws.len());
    }

    #[test]
    fn test_inactive_emitter_produces_no_draws() {
        let mut world = World::new();

        let tex = test_texture();
        let emitter = ParticleEmitter::new(100.0, tex);

        let e = world.spawn();
        world.add_component(e, emitter);

        // Deactivate via mutable access.
        world
            .get_by_index_mut::<ParticleEmitter>(e.index())
            .unwrap()
            .active = false;

        world.insert_resource(ParticleSystem::new());
        world.insert_resource(ParticleDrawBuffer::new());
        world.insert_resource(AnimationTime { dt: 1.0 });

        // Create a minimal bridge (no GPU needed for inactive emitters).
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .unwrap();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("test_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bridge = TextureBridge::new(&device, &queue, layout);

        update_particles(&mut world, &bridge);

        let buf = world.get_resource::<ParticleDrawBuffer>().unwrap();
        assert!(buf.draws.is_empty());
    }

    #[test]
    fn test_burst_emitter() {
        let mut world = World::new();

        let tex = test_texture();
        let emitter = ParticleEmitter::new(0.0, tex)
            .with_burst(25)
            .with_max_particles(100)
            .with_lifetime(5.0..5.0)
            .with_speed(0.0..0.0)
            .with_angle(0.0..0.0)
            .with_size(4.0..4.0);

        let e = world.spawn();
        world.add_component(e, emitter);
        world.insert_resource(ParticleSystem::new());
        world.insert_resource(ParticleDrawBuffer::new());
        world.insert_resource(AnimationTime { dt: 0.016 });

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .unwrap();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("test_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bridge = TextureBridge::new(&device, &queue, layout);

        // First frame: burst fires.
        update_particles(&mut world, &bridge);

        let buf = world.get_resource::<ParticleDrawBuffer>().unwrap();
        assert_eq!(buf.draws.len(), 25);

        // Second frame: burst does not fire again.
        world
            .get_resource_mut::<ParticleDrawBuffer>()
            .unwrap()
            .draws
            .clear();
        update_particles(&mut world, &bridge);

        let buf = world.get_resource::<ParticleDrawBuffer>().unwrap();
        assert_eq!(buf.draws.len(), 25); // Same particles, not re-burst.
    }

    #[test]
    fn test_particles_die_after_lifetime() {
        let mut world = World::new();

        let tex = test_texture();
        let emitter = ParticleEmitter::new(0.0, tex)
            .with_burst(10)
            .with_max_particles(100)
            .with_lifetime(1.0..1.0)
            .with_speed(0.0..0.0)
            .with_angle(0.0..0.0)
            .with_size(4.0..4.0);

        let e = world.spawn();
        world.add_component(e, emitter);
        world.insert_resource(ParticleSystem::new());
        world.insert_resource(ParticleDrawBuffer::new());

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .unwrap();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("test_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bridge = TextureBridge::new(&device, &queue, layout);

        // Spawn 10 particles.
        world.insert_resource(AnimationTime { dt: 0.016 });
        update_particles(&mut world, &bridge);

        let buf = world.get_resource::<ParticleDrawBuffer>().unwrap();
        assert_eq!(buf.draws.len(), 10);

        // Advance past lifetime (dt = 1.1 > lifetime 1.0).
        world.insert_resource(AnimationTime { dt: 1.1 });
        update_particles(&mut world, &bridge);

        let buf = world.get_resource::<ParticleDrawBuffer>().unwrap();
        assert!(
            buf.draws.is_empty(),
            "expected 0 draws after lifetime, got {}",
            buf.draws.len()
        );
    }
}
