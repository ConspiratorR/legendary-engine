//! Mathf utility struct (matches Unity's Mathf).

use std::f32::consts::PI;

/// Mathf constants and methods (matches Unity's `Mathf`).
#[allow(non_upper_case_globals, non_camel_case_types)]
pub struct Mathf;

impl Mathf {
    pub const PI: f32 = PI;
    pub const Epsilon: f32 = f32::EPSILON;
    pub const Infinity: f32 = f32::INFINITY;
    pub const NegativeInfinity: f32 = f32::NEG_INFINITY;
    pub const Deg2Rad: f32 = PI / 180.0;
    pub const Rad2Deg: f32 = 180.0 / PI;

    pub fn Lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t.clamp(0.0, 1.0)
    }

    pub fn LerpUnclamped(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    pub fn LerpAngle(a: f32, b: f32, t: f32) -> f32 {
        let delta = Self::Repeat(b - a, 360.0);
        let delta = if delta > 180.0 { delta - 360.0 } else { delta };
        a + delta * t.clamp(0.0, 1.0)
    }

    pub fn InverseLerp(a: f32, b: f32, value: f32) -> f32 {
        if (a - b).abs() < Self::Epsilon { return 0.0; }
        ((value - a) / (b - a)).clamp(0.0, 1.0)
    }

    pub fn SmoothStep(from: f32, to: f32, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        let t = -2.0 * t * t * t + 3.0 * t * t;
        from + (to - from) * t
    }

    pub fn MoveTowards(current: f32, target: f32, max_delta: f32) -> f32 {
        let diff = target - current;
        if diff.abs() <= max_delta { target }
        else { current + diff.signum() * max_delta }
    }

    pub fn MoveTowardsAngle(current: f32, target: f32, max_delta: f32) -> f32 {
        let delta = Self::DeltaAngle(current, target);
        if -max_delta < delta && delta < max_delta { target }
        else { Self::MoveTowards(current, target, max_delta) }
    }

    pub fn SmoothDamp(current: f32, target: f32, velocity: &mut f32, smooth_time: f32, max_speed: f32, delta_time: f32) -> f32 {
        let smooth_time = smooth_time.max(0.0001);
        let omega = 2.0 / smooth_time;
        let x = omega * delta_time;
        let exp = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
        let change = current - target;
        let original_to = target;

        let max_change = max_speed * smooth_time;
        let change = change.clamp(-max_change, max_change);
        let temp = (*velocity + omega * change) * delta_time;
        *velocity = (*velocity - omega * temp) * exp;
        let output = target + (change + temp) * exp;

        if (original_to - current > 0.0) == (output > original_to) {
            *velocity = (original_to - output) / delta_time;
            original_to
        } else {
            output
        }
    }

    pub fn Approximately(a: f32, b: f32) -> bool {
        (b - a).abs() < Self::Epsilon.max(a.abs() * Self::Epsilon)
    }

    pub fn DeltaAngle(current: f32, target: f32) -> f32 {
        let mut delta = Self::Repeat(target - current, 360.0);
        if delta > 180.0 { delta - 360.0 } else { delta }
    }

    pub fn PingPong(t: f32, length: f32) -> f32 {
        let t = Self::Repeat(t, length * 2.0);
        length - (t - length).abs()
    }

    pub fn Repeat(t: f32, length: f32) -> f32 {
        t - (t / length).floor() * length
    }

    pub fn ClosestPowerOfTwo(value: i32) -> i32 {
        if value <= 1 { return 1; }
        let upper = Self::NextPowerOfTwo(value);
        let lower = upper >> 1;
        if (value - lower).abs() <= (upper - value).abs() { lower } else { upper }
    }

    pub fn NextPowerOfTwo(value: i32) -> i32 {
        if value <= 0 { return 1; }
        let mut v = value;
        v -= 1;
        v |= v >> 1;
        v |= v >> 2;
        v |= v >> 4;
        v |= v >> 8;
        v |= v >> 16;
        v + 1
    }

    pub fn IsPowerOfTwo(value: i32) -> bool {
        value > 0 && (value & (value - 1)) == 0
    }

    pub fn GammaToLinearSpace(value: f32) -> f32 {
        if value <= 0.04045 { value / 12.92 }
        else { ((value + 0.055) / 1.055).powf(2.4) }
    }

    pub fn LinearToGammaSpace(value: f32) -> f32 {
        if value <= 0.0031308 { value * 12.92 }
        else { 1.055 * value.powf(1.0 / 2.4) - 0.055 }
    }
}
