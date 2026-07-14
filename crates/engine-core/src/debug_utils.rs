//! Debug utilities (matches Unity's Debug, Gizmos).

use engine_math::Vec3;

/// Debug logging (matches Unity's `Debug`).
pub struct Debug;

impl Debug {
    pub fn Log(message: &str) {
        println!("[LOG] {}", message);
    }

    pub fn LogWarning(message: &str) {
        println!("[WARN] {}", message);
    }

    pub fn LogError(message: &str) {
        eprintln!("[ERROR] {}", message);
    }

    pub fn DrawRay(from: Vec3, direction: Vec3, color: [f32; 4], duration: f32) {
        let _ = (from, direction, color, duration);
    }

    pub fn DrawLine(start: Vec3, end: Vec3, color: [f32; 4], duration: f32) {
        let _ = (start, end, color, duration);
    }
}

/// Gizmos drawing (matches Unity's `Gizmos`).
pub struct Gizmos;

impl Gizmos {
    pub fn DrawSphere(center: Vec3, radius: f32) {
        let _ = (center, radius);
    }

    pub fn DrawCube(center: Vec3, size: Vec3) {
        let _ = (center, size);
    }

    pub fn DrawWireSphere(center: Vec3, radius: f32) {
        let _ = (center, radius);
    }

    pub fn DrawWireCube(center: Vec3, size: Vec3) {
        let _ = (center, size);
    }

    pub fn DrawLine(from: Vec3, to: Vec3) {
        let _ = (from, to);
    }

    pub fn DrawRay(from: Vec3, direction: Vec3) {
        let _ = (from, direction);
    }
}
