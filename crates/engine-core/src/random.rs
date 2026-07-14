//! Random utility (matches Unity's Random).

use engine_math::{Quat, Vec3};

/// Random number generator (matches Unity's `Random`).
pub struct Random;

impl Random {
    pub fn Range(min: f32, max: f32) -> f32 {
        let r = rand::random::<f32>();
        min + r * (max - min)
    }

    pub fn RangeInt(min: i32, max: i32) -> i32 {
        let r = rand::random::<u32>() as i32;
        min + (r.abs() % (max - min + 1))
    }

    pub fn InsideUnitSphere() -> Vec3 {
        loop {
            let v = Vec3::new(
                Self::Range(-1.0, 1.0),
                Self::Range(-1.0, 1.0),
                Self::Range(-1.0, 1.0),
            );
            if v.length_squared() <= 1.0 {
                return v;
            }
        }
    }

    pub fn OnUnitSphere() -> Vec3 {
        loop {
            let v = Vec3::new(
                Self::Range(-1.0, 1.0),
                Self::Range(-1.0, 1.0),
                Self::Range(-1.0, 1.0),
            );
            let len = v.length();
            if len > 0.001 && len <= 1.0 {
                return v / len;
            }
        }
    }

    pub fn Value() -> f32 {
        rand::random::<f32>()
    }

    pub fn Rotation() -> Quat {
        let u1 = Self::Range(0.0, 1.0);
        let u2 = Self::Range(0.0, 1.0);
        let u3 = Self::Range(0.0, 1.0);
        let sqrt1m1 = (1.0 - u1).sqrt();
        Quat::from_xyzw(
            sqrt1m1 * (2.0 * std::f32::consts::PI * u2).sin(),
            sqrt1m1 * (2.0 * std::f32::consts::PI * u2).cos(),
            u1.sqrt() * (2.0 * std::f32::consts::PI * u3).sin(),
            u1.sqrt() * (2.0 * std::f32::consts::PI * u3).cos(),
        )
    }
}
