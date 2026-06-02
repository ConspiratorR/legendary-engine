#![no_main]
use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;
use engine_physics::contact::{ContactSolver, ContactManifold, ContactPoint};
use engine_math::Vec3;

#[derive(Arbitrary, Debug)]
struct FuzzContact {
    position: [f32; 3],
    normal: [f32; 3],
    depth: f32,
}

#[derive(Arbitrary, Debug)]
struct FuzzManifold {
    contacts: Vec<FuzzContact>,
    restitution: f32,
    friction: f32,
}

#[derive(Arbitrary, Debug)]
struct FuzzSolverInput {
    manifolds: Vec<FuzzManifold>,
    vel_a: [f32; 3],
    vel_b: [f32; 3],
    inv_mass_a: f32,
    inv_mass_b: f32,
    dt: f32,
    iterations: u32,
}

fn sanitize_f32(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

fn sanitize_vec3(v: [f32; 3]) -> Vec3 {
    Vec3::new(sanitize_f32(v[0]), sanitize_f32(v[1]), sanitize_f32(v[2]))
}

fuzz_target!(|input: FuzzSolverInput| {
    let iterations = (input.iterations % 20).max(1);
    let solver = ContactSolver {
        iterations,
        baumgarte: 0.2,
        slop: 0.005,
    };

    for fuzz_m in input.manifolds.iter().take(4) {
        let mut manifold = ContactManifold::new(0, 1);
        manifold.restitution = sanitize_f32(fuzz_m.restitution).clamp(0.0, 1.0);
        manifold.friction = sanitize_f32(fuzz_m.friction).clamp(0.0, 2.0);

        for fuzz_c in fuzz_m.contacts.iter().take(4) {
            let normal = sanitize_vec3(fuzz_c.normal);
            let normal_len = normal.length();
            let normal = if normal_len > 0.001 {
                normal / normal_len
            } else {
                Vec3::Y
            };
            let depth = sanitize_f32(fuzz_c.depth).abs().max(0.001);

            manifold.add_contact(ContactPoint::new(
                sanitize_vec3(fuzz_c.position),
                normal,
                depth,
            ));
        }

        if manifold.contacts.is_empty() {
            continue;
        }

        let mut vel_a = sanitize_vec3(input.vel_a);
        let mut vel_b = sanitize_vec3(input.vel_b);
        let inv_mass_a = sanitize_f32(input.inv_mass_a).abs().max(0.0);
        let inv_mass_b = sanitize_f32(input.inv_mass_b).abs().max(0.0);
        let dt = sanitize_f32(input.dt).abs().max(0.001);

        solver.solve_manifold(
            &mut manifold,
            &mut vel_a,
            &mut vel_b,
            inv_mass_a,
            inv_mass_b,
            dt,
        );

        // Verify: accumulated impulses must be non-negative
        for contact in &manifold.contacts {
            assert!(
                contact.accumulated_normal_impulse >= -0.01,
                "Negative accumulated normal impulse: {}",
                contact.accumulated_normal_impulse,
            );
        }

        // Verify: velocities must remain finite
        assert!(vel_a.is_finite(), "vel_a not finite: {:?}", vel_a);
        assert!(vel_b.is_finite(), "vel_b not finite: {:?}", vel_b);
    }
});
