use engine_math::Vec3;

/// Continuous Collision Detection (CCD) for fast-moving objects.
///
/// Standard discrete collision detection can miss collisions when objects
/// move too fast (tunneling). CCD sweeps the shape along its trajectory
/// to find the earliest time of impact (TOI).
///
/// Result of a CCD sweep test.
#[derive(Debug, Clone, Copy)]
pub struct SweepResult {
    /// Time of impact in [0, 1] (0 = start, 1 = end of movement).
    pub toi: f32,
    /// Contact normal at impact.
    pub normal: Vec3,
    /// Contact point at impact.
    pub point: Vec3,
    /// Whether a collision was detected.
    pub hit: bool,
}

impl SweepResult {
    pub fn miss() -> Self {
        Self {
            toi: 1.0,
            normal: Vec3::ZERO,
            point: Vec3::ZERO,
            hit: false,
        }
    }
}

/// Sweep a sphere along a line segment and test against a static sphere.
///
/// Returns the earliest time of impact if the moving sphere would hit
/// the static sphere during its movement from `start` to `end`.
pub fn sweep_sphere_sphere(
    start: Vec3,
    end: Vec3,
    radius_a: f32,
    center_b: Vec3,
    radius_b: f32,
) -> SweepResult {
    let direction = end - start;
    let dist = direction.length();
    if dist < f32::EPSILON {
        // No movement — use discrete test
        let delta = start - center_b;
        let d = delta.length();
        let sum_r = radius_a + radius_b;
        if d < sum_r {
            return SweepResult {
                toi: 0.0,
                normal: if d > f32::EPSILON { delta / d } else { Vec3::Y },
                point: start,
                hit: true,
            };
        }
        return SweepResult::miss();
    }

    let dir = direction / dist;
    let sum_r = radius_a + radius_b;

    // Ray-sphere intersection: solve |start + t*dir - center_b|^2 = sum_r^2
    let oc = start - center_b;
    let a = dir.dot(dir); // = 1.0 for normalized dir
    let b = 2.0 * oc.dot(dir);
    let c = oc.dot(oc) - sum_r * sum_r;

    // Check if already overlapping at start (c <= 0)
    if c <= 0.0 {
        let delta = start - center_b;
        let d = delta.length();
        return SweepResult {
            toi: 0.0,
            normal: if d > f32::EPSILON { delta / d } else { Vec3::Y },
            point: start,
            hit: true,
        };
    }

    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return SweepResult::miss();
    }

    let sqrt_disc = discriminant.sqrt();
    let t = (-b - sqrt_disc) / (2.0 * a);

    // t is in units of dist, normalize to [0, 1]
    let toi = t / dist;

    if !(0.0..=1.0).contains(&toi) {
        return SweepResult::miss();
    }

    let hit_point = start + dir * t;
    let normal = (hit_point - center_b).normalize_or_zero();

    SweepResult {
        toi,
        normal,
        point: hit_point,
        hit: true,
    }
}

/// Sweep a sphere against an AABB (axis-aligned bounding box).
///
/// Returns the earliest time of impact.
pub fn sweep_sphere_aabb(
    start: Vec3,
    end: Vec3,
    radius: f32,
    aabb_min: Vec3,
    aabb_max: Vec3,
) -> SweepResult {
    // Expand AABB by sphere radius
    let expanded_min = aabb_min - Vec3::new(radius, radius, radius);
    let expanded_max = aabb_max + Vec3::new(radius, radius, radius);

    // Ray-AABB intersection (slab method)
    let direction = end - start;
    let mut t_min: f32 = 0.0;
    let mut t_max: f32 = 1.0;

    let mut hit_normal = Vec3::ZERO;

    for axis in 0..3 {
        let (d, s, lo, hi) = match axis {
            0 => (direction.x, start.x, expanded_min.x, expanded_max.x),
            1 => (direction.y, start.y, expanded_min.y, expanded_max.y),
            _ => (direction.z, start.z, expanded_min.z, expanded_max.z),
        };

        if d.abs() < f32::EPSILON {
            // Ray is parallel to this slab — check if origin is inside
            if s < lo || s > hi {
                return SweepResult::miss();
            }
        } else {
            let inv_d = 1.0 / d;
            let mut t0 = (lo - s) * inv_d;
            let mut t1 = (hi - s) * inv_d;

            let (mut normal_neg, mut normal_pos) = match axis {
                0 => (Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
                1 => (Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 1.0, 0.0)),
                _ => (Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 0.0, 1.0)),
            };

            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
                std::mem::swap(&mut normal_neg, &mut normal_pos);
            }

            if t0 > t_min {
                t_min = t0;
                hit_normal = normal_neg;
            }
            if t1 < t_max {
                t_max = t1;
            }

            if t_min > t_max {
                return SweepResult::miss();
            }
        }
    }

    if (0.0..=1.0).contains(&t_min) {
        let hit_point = start + direction * t_min;
        SweepResult {
            toi: t_min,
            normal: hit_normal,
            point: hit_point,
            hit: true,
        }
    } else {
        SweepResult::miss()
    }
}

/// CCD component — marks an entity as requiring continuous collision detection.
#[derive(Debug, Clone)]
pub struct CcdBody {
    /// Whether CCD is enabled for this body.
    pub enabled: bool,
    /// Minimum speed threshold to activate CCD (avoid unnecessary sweeps).
    pub activation_threshold: f32,
}

impl Default for CcdBody {
    fn default() -> Self {
        Self {
            enabled: true,
            activation_threshold: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sweep_sphere_sphere_hit() {
        // Sphere moving from origin toward a sphere at (5, 0, 0) with radius 1
        let result = sweep_sphere_sphere(
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            0.5,
            Vec3::new(5.0, 0.0, 0.0),
            1.0,
        );
        assert!(result.hit);
        assert!(result.toi > 0.0 && result.toi < 1.0);
        assert!(result.normal.x < 0.0); // Normal should point back toward A
    }

    #[test]
    fn test_sweep_sphere_sphere_miss() {
        let result = sweep_sphere_sphere(
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            0.5,
            Vec3::new(0.0, 5.0, 0.0),
            1.0,
        );
        assert!(!result.hit);
    }

    #[test]
    fn test_sweep_sphere_sphere_already_overlapping() {
        let result = sweep_sphere_sphere(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            1.0,
            Vec3::new(0.5, 0.0, 0.0),
            1.0,
        );
        assert!(result.hit);
        assert!((result.toi).abs() < 1e-6); // Hit at start
    }

    #[test]
    fn test_sweep_sphere_sphere_no_movement() {
        let result =
            sweep_sphere_sphere(Vec3::ZERO, Vec3::ZERO, 1.0, Vec3::new(0.5, 0.0, 0.0), 1.0);
        assert!(result.hit); // Overlapping at rest
    }

    #[test]
    fn test_sweep_sphere_aabb_hit() {
        // Sphere at origin moving toward a box at (5, 0, 0)
        let result = sweep_sphere_aabb(
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            0.5,
            Vec3::new(4.0, -1.0, -1.0),
            Vec3::new(6.0, 1.0, 1.0),
        );
        assert!(result.hit);
        assert!(result.toi > 0.0 && result.toi < 1.0);
    }

    #[test]
    fn test_sweep_sphere_aabb_miss() {
        let result = sweep_sphere_aabb(
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            0.5,
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(1.0, 6.0, 1.0),
        );
        assert!(!result.hit);
    }

    #[test]
    fn test_sweep_result_miss() {
        let r = SweepResult::miss();
        assert!(!r.hit);
        assert!((r.toi - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_ccd_body_default() {
        let ccd = CcdBody::default();
        assert!(ccd.enabled);
        assert!((ccd.activation_threshold - 1.0).abs() < 1e-6);
    }
}
