//! Benchmark for script bridge call overhead.
//!
//! Target: < 1μs per bridge call.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use engine_core::color::Color;
use engine_core::transform::Transform;
use engine_ecs::world::World;
use engine_math::{Quat, Vec2, Vec3, Vec4};
use engine_script::callback::{CallbackArg, CallbackRegistry, CallbackResult};
use engine_script::type_registry::TypeRegistry;
use mlua::prelude::*;
use std::sync::{Arc, RwLock};

fn bench_type_registry_lua_get(c: &mut Criterion) {
    let registry = TypeRegistry::default();
    let lua = Lua::new();
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Vec3::new(1.0, 2.0, 3.0));

    c.bench_function("type_registry_lua_get_vec3", |b| {
        b.iter(|| {
            let val = registry.lua_get(&lua, &world, "Vec3", e.index()).unwrap();
            black_box(val);
        })
    });
}

fn bench_type_registry_wasm_get(c: &mut Criterion) {
    let registry = TypeRegistry::default();
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Vec3::new(1.0, 2.0, 3.0));

    c.bench_function("type_registry_wasm_get_vec3", |b| {
        b.iter(|| {
            let mut buf = [0u8; 12];
            let written = registry.wasm_get(&world, "Vec3", e.index(), &mut buf);
            black_box((written, buf));
        })
    });
}

fn bench_type_registry_wasm_set(c: &mut Criterion) {
    let registry = TypeRegistry::default();
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Vec3::new(0.0, 0.0, 0.0));
    let bytes = [0u8, 0, 0, 63, 0, 0, 0, 64, 0, 0, 64, 64]; // 1.0, 2.0, 3.0

    c.bench_function("type_registry_wasm_set_vec3", |b| {
        b.iter(|| {
            let ok = registry.wasm_set(&mut world, "Vec3", e.index(), &bytes);
            black_box(ok);
        })
    });
}

fn bench_vec3_bytes_roundtrip(c: &mut Criterion) {
    use engine_script::type_registry::{bytes_to_vec3, vec3_to_bytes};

    let v = Vec3::new(1.0, 2.0, 3.0);

    c.bench_function("vec3_bytes_roundtrip", |b| {
        b.iter(|| {
            let bytes = vec3_to_bytes(&v);
            let v2 = bytes_to_vec3(&bytes);
            black_box(v2);
        })
    });
}

fn bench_color_bytes_roundtrip(c: &mut Criterion) {
    use engine_script::type_registry::{bytes_to_color, color_to_bytes};

    let color = Color::new(0.5, 0.6, 0.7, 0.8);

    c.bench_function("color_bytes_roundtrip", |b| {
        b.iter(|| {
            let bytes = color_to_bytes(&color);
            let c2 = bytes_to_color(&bytes);
            black_box(c2);
        })
    });
}

fn bench_transform_bytes_roundtrip(c: &mut Criterion) {
    use engine_script::type_registry::{bytes_to_transform, transform_to_bytes};

    let tr = Transform {
        position: Vec3::new(1.0, 2.0, 3.0),
        rotation: Vec3::new(0.1, 0.2, 0.3),
        scale: Vec3::ONE,
    };

    c.bench_function("transform_bytes_roundtrip", |b| {
        b.iter(|| {
            let bytes = transform_to_bytes(&tr);
            let tr2 = bytes_to_transform(&bytes);
            black_box(tr2);
        })
    });
}

fn bench_callback_invoke(c: &mut Criterion) {
    let mut reg = CallbackRegistry::new();
    reg.register("noop", |_| Ok(CallbackResult::None));

    c.bench_function("callback_invoke", |b| {
        b.iter(|| {
            let result = reg.invoke("noop", &[]).unwrap();
            black_box(result);
        })
    });
}

fn bench_callback_invoke_with_args(c: &mut Criterion) {
    let mut reg = CallbackRegistry::new();
    reg.register("add", |args| {
        let a = args[0].as_f64().unwrap_or(0.0);
        let b = args[1].as_f64().unwrap_or(0.0);
        Ok(CallbackResult::F64(a + b))
    });

    c.bench_function("callback_invoke_2args", |b| {
        b.iter(|| {
            let result = reg
                .invoke("add", &[CallbackArg::F64(3.0), CallbackArg::F64(4.0)])
                .unwrap();
            black_box(result);
        })
    });
}

fn bench_lua_callback_through_bridge(c: &mut Criterion) {
    let mut reg = CallbackRegistry::new();
    reg.register("double", |args| {
        let v = args[0].as_f64().unwrap_or(0.0);
        Ok(CallbackResult::F64(v * 2.0))
    });
    let reg = Arc::new(RwLock::new(reg));

    let lua = Lua::new();
    let cb_fn = CallbackRegistry::create_lua_function(reg);
    lua.globals()
        .set("callback", lua.create_function(cb_fn).unwrap())
        .unwrap();

    c.bench_function("lua_callback_through_bridge", |b| {
        b.iter(|| {
            let result: f64 = lua.load("return callback('double', 21.0)").eval().unwrap();
            black_box(result);
        })
    });
}

fn bench_wasm_component_bridge_get(c: &mut Criterion) {
    use engine_script::wasm::WasmComponentBridge;

    let mut bridge = WasmComponentBridge::new();
    bridge.register::<Vec3>("Position", 12, |v| v3_to_bytes(v), |b| bytes_to_v3(b));

    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Vec3::new(1.0, 2.0, 3.0));

    c.bench_function("wasm_bridge_get_vec3", |b| {
        b.iter(|| {
            let mut buf = [0u8; 12];
            let written = bridge.get(&world, "Position", e.index(), &mut buf);
            black_box((written, buf));
        })
    });
}

// Helpers for WASM bridge benchmark
fn v3_to_bytes(v: &Vec3) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(12);
    bytes.extend_from_slice(&v.x.to_le_bytes());
    bytes.extend_from_slice(&v.y.to_le_bytes());
    bytes.extend_from_slice(&v.z.to_le_bytes());
    bytes
}

fn bytes_to_v3(bytes: &[u8]) -> Vec3 {
    Vec3::new(
        f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
    )
}

criterion_group!(
    bridge_benches,
    bench_type_registry_lua_get,
    bench_type_registry_wasm_get,
    bench_type_registry_wasm_set,
    bench_vec3_bytes_roundtrip,
    bench_color_bytes_roundtrip,
    bench_transform_bytes_roundtrip,
    bench_callback_invoke,
    bench_callback_invoke_with_args,
    bench_lua_callback_through_bridge,
    bench_wasm_component_bridge_get,
);

criterion_main!(bridge_benches);
