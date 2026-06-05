use criterion::{black_box, criterion_group, criterion_main, Criterion};
use glam::{Mat4, Quat, Vec3};

fn bench_vec3_add(c: &mut Criterion) {
    let a = black_box(Vec3::new(1.0, 2.0, 3.0));
    let b = black_box(Vec3::new(4.0, 5.0, 6.0));
    c.bench_function("vec3_add", |bench| bench.iter(|| a + b));
}

fn bench_vec3_dot(c: &mut Criterion) {
    let a = black_box(Vec3::new(1.0, 2.0, 3.0));
    let b = black_box(Vec3::new(4.0, 5.0, 6.0));
    c.bench_function("vec3_dot", |bench| bench.iter(|| a.dot(b)));
}

fn bench_vec3_cross(c: &mut Criterion) {
    let a = black_box(Vec3::new(1.0, 2.0, 3.0));
    let b = black_box(Vec3::new(4.0, 5.0, 6.0));
    c.bench_function("vec3_cross", |bench| bench.iter(|| a.cross(b)));
}

fn bench_vec3_normalize(c: &mut Criterion) {
    let v = black_box(Vec3::new(3.0, 4.0, 5.0));
    c.bench_function("vec3_normalize", |bench| bench.iter(|| v.normalize()));
}

fn bench_mat4_mul_vec3(c: &mut Criterion) {
    let m = black_box(Mat4::IDENTITY);
    let v = black_box(Vec3::new(1.0, 2.0, 3.0));
    c.bench_function("mat4_mul_vec3", |bench| bench.iter(|| m.transform_point3(v)));
}

fn bench_mat4_inverse(c: &mut Criterion) {
    let m = black_box(Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)));
    c.bench_function("mat4_inverse", |bench| bench.iter(|| m.inverse()));
}

fn bench_mat4_mul_mat4(c: &mut Criterion) {
    let a = black_box(Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)));
    let b = black_box(Mat4::from_rotation_y(1.5));
    c.bench_function("mat4_mul_mat4", |bench| bench.iter(|| a * b));
}

fn bench_quat_mul(c: &mut Criterion) {
    let a = black_box(Quat::from_rotation_y(1.0));
    let b = black_box(Quat::from_rotation_x(0.5));
    c.bench_function("quat_mul", |bench| bench.iter(|| a * b));
}

fn bench_quat_slerp(c: &mut Criterion) {
    let a = black_box(Quat::IDENTITY);
    let b = black_box(Quat::from_rotation_y(std::f32::consts::PI));
    c.bench_function("quat_slerp", |bench| bench.iter(|| a.slerp(b, 0.5)));
}

criterion_group!(
    benches,
    bench_vec3_add,
    bench_vec3_dot,
    bench_vec3_cross,
    bench_vec3_normalize,
    bench_mat4_mul_vec3,
    bench_mat4_inverse,
    bench_mat4_mul_mat4,
    bench_quat_mul,
    bench_quat_slerp,
);
criterion_main!(benches);
