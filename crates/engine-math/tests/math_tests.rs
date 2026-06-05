use engine_math::*;
use glam::{Mat4, Quat, Vec3};

const EPSILON: f32 = 1e-6;

#[test]
fn normalize_zero_vector() {
    let v = Vec3::ZERO;
    let n = v.normalize();
    // glam returns NaN for normalize of zero vector
    assert!(n.x.is_nan() && n.y.is_nan() && n.z.is_nan());
}

#[test]
fn normalize_unit_vector() {
    let v = Vec3::new(0.0, 1.0, 0.0);
    let n = v.normalize();
    assert!((n.x).abs() < EPSILON);
    assert!((n.y - 1.0).abs() < EPSILON);
    assert!((n.z).abs() < EPSILON);
    assert!((n.length() - 1.0).abs() < EPSILON);
}

#[test]
fn normalize_arbitrary_vector() {
    let v = Vec3::new(3.0, 4.0, 0.0);
    let n = v.normalize();
    assert!((n.length() - 1.0).abs() < EPSILON);
    assert!((n.x - 0.6).abs() < EPSILON);
    assert!((n.y - 0.8).abs() < EPSILON);
}

#[test]
fn matrix_inverse_identity() {
    let m = Mat4::IDENTITY;
    let inv = m.inverse();
    assert!(inv.is_finite());
    // Identity inverse should be identity
    for i in 0..4 {
        for j in 0..4 {
            let expected = if i == j { 1.0 } else { 0.0 };
            assert!(
                (inv.col(i)[j] - expected).abs() < EPSILON,
                "inv[{}][{}] = {}, expected {}",
                i,
                j,
                inv.col(i)[j],
                expected
            );
        }
    }
}

#[test]
fn matrix_inverse_translation() {
    let m = Mat4::from_translation(Vec3::new(5.0, -3.0, 7.0));
    let inv = m.inverse();
    assert!(inv.is_finite());
    // Applying m then inv should give identity
    let product = m * inv;
    let identity = Mat4::IDENTITY;
    for i in 0..4 {
        for j in 0..4 {
            assert!(
                (product.col(i)[j] - identity.col(i)[j]).abs() < EPSILON,
                "m * m_inv[{}][{}] = {}, expected {}",
                i,
                j,
                product.col(i)[j],
                identity.col(i)[j]
            );
        }
    }
}

#[test]
fn quat_slerp_same_rotation() {
    let q = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4);
    let result = q.slerp(q, 0.5);
    // Slerp between identical quaternions should return the same quaternion
    assert!((result.x - q.x).abs() < EPSILON);
    assert!((result.y - q.y).abs() < EPSILON);
    assert!((result.z - q.z).abs() < EPSILON);
    assert!((result.w - q.w).abs() < EPSILON);
}

#[test]
fn quat_slerp_endpoints() {
    let q0 = Quat::IDENTITY;
    let q1 = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    let at_start = q0.slerp(q1, 0.0);
    let at_end = q0.slerp(q1, 1.0);
    // slerp(t=0) should be q0
    assert!((at_start.x - q0.x).abs() < EPSILON);
    assert!((at_start.y - q0.y).abs() < EPSILON);
    assert!((at_start.z - q0.z).abs() < EPSILON);
    assert!((at_start.w - q0.w).abs() < EPSILON);
    // slerp(t=1) should be q1 (or -q1 due to double cover; use dot product)
    let dot = at_end.dot(q1).abs();
    assert!(dot > 1.0 - EPSILON, "at_end dot q1 = {}", dot);
}

#[test]
fn vec3_cross_product_parallel() {
    let a = Vec3::new(1.0, 0.0, 0.0);
    let b = Vec3::new(2.0, 0.0, 0.0);
    let c = a.cross(b);
    // Cross product of parallel vectors should be zero
    assert!(c.x.abs() < EPSILON);
    assert!(c.y.abs() < EPSILON);
    assert!(c.z.abs() < EPSILON);
}

#[test]
fn vec3_cross_product_orthogonal() {
    let x = Vec3::X;
    let y = Vec3::Y;
    let z = x.cross(y);
    assert!((z.x - 0.0).abs() < EPSILON);
    assert!((z.y - 0.0).abs() < EPSILON);
    assert!((z.z - 1.0).abs() < EPSILON);
}

#[test]
fn vec3_dot_product_orthogonal() {
    let x = Vec3::X;
    let y = Vec3::Y;
    assert!(x.dot(y).abs() < EPSILON);
}

#[test]
fn vec3_dot_product_parallel() {
    let a = Vec3::new(3.0, 0.0, 0.0);
    let b = Vec3::new(5.0, 0.0, 0.0);
    assert!((a.dot(b) - 15.0).abs() < EPSILON);
}

#[test]
fn extend_with_w_values() {
    let v = Vec3::new(1.0, 2.0, 3.0);
    assert_eq!(v.extend_with_w(0.0), Vec4::new(1.0, 2.0, 3.0, 0.0));
    assert_eq!(v.extend_with_w(1.0), Vec4::new(1.0, 2.0, 3.0, 1.0));
    assert_eq!(v.extend_with_w(-1.0), Vec4::new(1.0, 2.0, 3.0, -1.0));
}

#[test]
fn look_at_lh_produces_finite_matrix() {
    let eye = Vec3::new(0.0, 5.0, -10.0);
    let target = Vec3::ZERO;
    let up = Vec3::Y;
    let m = Mat4::look_at_lh(eye, target, up);
    for i in 0..4 {
        for j in 0..4 {
            assert!(m.col(i)[j].is_finite(), "m[{}][{}] is not finite", i, j);
        }
    }
}
