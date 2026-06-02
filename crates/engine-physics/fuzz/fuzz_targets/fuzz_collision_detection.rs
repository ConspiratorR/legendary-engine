#![no_main]
use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;
use engine_physics::collider::{
    Collider, ColliderShape, check_collision, check_sphere_sphere, check_sphere_box,
    check_obb_obb, check_sphere_capsule, check_capsule_capsule,
};
use engine_math::{Quat, Vec3};

#[derive(Arbitrary, Debug)]
enum CollisionTest {
    SphereSphere {
        p1: [f32; 3], r1: f32,
        p2: [f32; 3], r2: f32,
    },
    SphereBox {
        sp: [f32; 3], sr: f32,
        bp: [f32; 3], bh: [f32; 3],
    },
    ObbObb {
        p1: [f32; 3], h1: [f32; 3],
        p2: [f32; 3], h2: [f32; 3],
    },
    SphereCapsule {
        sp: [f32; 3], sr: f32,
        cp: [f32; 3], cr: f32, ch: f32,
    },
    CapsuleCapsule {
        p1: [f32; 3], r1: f32, h1: f32,
        p2: [f32; 3], r2: f32, h2: f32,
    },
    Dispatcher {
        p1: [f32; 3], p2: [f32; 3],
        shape_a: u8, shape_b: u8,
    },
}

fn sanitize_f32(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

fn sanitize_vec3(v: [f32; 3]) -> Vec3 {
    Vec3::new(sanitize_f32(v[0]), sanitize_f32(v[1]), sanitize_f32(v[2]))
}

fuzz_target!(|test: CollisionTest| {
    match test {
        CollisionTest::SphereSphere { p1, r1, p2, r2 } => {
            let r1 = sanitize_f32(r1).abs().max(0.001);
            let r2 = sanitize_f32(r2).abs().max(0.001);
            let result = check_sphere_sphere(sanitize_vec3(p1), r1, sanitize_vec3(p2), r2);
            // Verify: if collision detected, depth must be positive
            if let Some(info) = result {
                assert!(info.depth > 0.0, "depth must be positive: {}", info.depth);
                // Normal should be approximately unit length
                let len = info.normal.length();
                assert!(len > 0.9 && len < 1.1, "normal not unit: {}", len);
            }
        }
        CollisionTest::SphereBox { sp, sr, bp, bh } => {
            let sr = sanitize_f32(sr).abs().max(0.001);
            let bh = sanitize_vec3(bh);
            let bh = Vec3::new(bh.x.abs().max(0.001), bh.y.abs().max(0.001), bh.z.abs().max(0.001));
            let result = check_sphere_box(sanitize_vec3(sp), sr, sanitize_vec3(bp), bh);
            if let Some(info) = result {
                assert!(info.depth > 0.0, "depth must be positive: {}", info.depth);
            }
        }
        CollisionTest::ObbObb { p1, h1, p2, h2 } => {
            let h1 = sanitize_vec3(h1);
            let h1 = Vec3::new(h1.x.abs().max(0.001), h1.y.abs().max(0.001), h1.z.abs().max(0.001));
            let h2 = sanitize_vec3(h2);
            let h2 = Vec3::new(h2.x.abs().max(0.001), h2.y.abs().max(0.001), h2.z.abs().max(0.001));
            let result = check_obb_obb(
                sanitize_vec3(p1), Quat::IDENTITY, h1,
                sanitize_vec3(p2), Quat::IDENTITY, h2,
            );
            if let Some(info) = result {
                assert!(info.depth > 0.0, "depth must be positive: {}", info.depth);
            }
        }
        CollisionTest::SphereCapsule { sp, sr, cp, cr, ch } => {
            let sr = sanitize_f32(sr).abs().max(0.001);
            let cr = sanitize_f32(cr).abs().max(0.001);
            let ch = sanitize_f32(ch).abs().max(0.001);
            let result = check_sphere_capsule(
                sanitize_vec3(sp), sr,
                sanitize_vec3(cp), Quat::IDENTITY, cr, ch,
            );
            if let Some(info) = result {
                assert!(info.depth > 0.0, "depth must be positive: {}", info.depth);
            }
        }
        CollisionTest::CapsuleCapsule { p1, r1, h1, p2, r2, h2 } => {
            let r1 = sanitize_f32(r1).abs().max(0.001);
            let h1 = sanitize_f32(h1).abs().max(0.001);
            let r2 = sanitize_f32(r2).abs().max(0.001);
            let h2 = sanitize_f32(h2).abs().max(0.001);
            let result = check_capsule_capsule(
                sanitize_vec3(p1), Quat::IDENTITY, r1, h1,
                sanitize_vec3(p2), Quat::IDENTITY, r2, h2,
            );
            if let Some(info) = result {
                assert!(info.depth > 0.0, "depth must be positive: {}", info.depth);
            }
        }
        CollisionTest::Dispatcher { p1, p2, shape_a, shape_b } => {
            let shapes = [
                ColliderShape::Sphere { radius: 0.5 },
                ColliderShape::Box { half_extents: Vec3::splat(0.5) },
                ColliderShape::Capsule { radius: 0.3, height: 1.0 },
                ColliderShape::Cylinder { radius: 0.3, height: 1.0 },
            ];
            let col_a = Collider {
                shape: shapes[shape_a as usize % shapes.len()].clone(),
                ..Default::default()
            };
            let col_b = Collider {
                shape: shapes[shape_b as usize % shapes.len()].clone(),
                ..Default::default()
            };
            let result = check_collision(
                sanitize_vec3(p1), Quat::IDENTITY, &col_a,
                sanitize_vec3(p2), Quat::IDENTITY, &col_b,
            );
            if let Some(info) = result {
                assert!(info.depth > 0.0, "depth must be positive: {}", info.depth);
            }
        }
    }
});
