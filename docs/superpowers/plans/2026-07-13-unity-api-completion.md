# Unity API 补全实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 补全 RustEngine 中缺失的 Unity 风格 API，使引擎脚本层与 Unity 尽可能兼容

**Architecture:** 在 `engine-core` crate 中添加缺失的类型、组件和方法。遵循 Unity 的 PascalCase 命名约定（公共 API）和已有的代码模式。每个 Task 独立可测试。

**Tech Stack:** Rust, engine-core crate, glam (数学库)

---

## 文件结构

### 新增文件
- `crates/engine-core/src/raycast.rs` — Ray, RaycastHit, Physics 射线检测
- `crates/engine-core/src/bounds.rs` — Bounds, BoundsInt
- `crates/engine-core/src/layer_mask.rs` — LayerMask
- `crates/engine-core/src/physics_material.rs` — PhysicMaterial
- `crates/engine-core/src/contact.rs` — ContactPoint, Collision 数据
- `crates/engine-core/src/mathf.rs` — Mathf 结构体（常量 + 静态方法）
- `crates/engine-core/src/random.rs` — Random 工具类
- `crates/engine-core/src/debug_utils.rs` — Debug/Gizmos 工具
- `crates/engine-core/src/application.rs` — Application 信息
- `crates/engine-core/src/scene_management.rs` — SceneManager
- `crates/engine-core/src/character_controller.rs` — CharacterController 组件
- `crates/engine-core/tests/unity_api_completion_tests.rs` — 新增 API 的测试

### 修改文件
- `crates/engine-core/src/lib.rs` — 导出新模块
- `crates/engine-core/src/components.rs` — 添加缺失组件
- `crates/engine-core/src/transform.rs` — 补全 Transform 方法
- `crates/engine-core/src/monobehaviour.rs` — 补全生命周期回调
- `crates/engine-core/src/world.rs` — 补全 World 方法

---

## Phase 1: 基础数学类型（最高优先级）

### Task 1: Ray 和 RaycastHit

**Files:**
- Create: `crates/engine-core/src/raycast.rs`
- Modify: `crates/engine-core/src/lib.rs`
- Test: `crates/engine-core/tests/unity_api_completion_tests.rs`

- [ ] **Step 1: 创建 raycast.rs**

```rust
//! Ray casting types (matches Unity's Ray, RaycastHit, Physics).

use crate::component::Component;
use engine_math::{Vec3, Quat};
use std::any::Any;

/// A ray in 3D space (matches Unity's `Ray`).
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self { origin, direction: direction.normalize_or_zero() }
    }

    pub fn GetPoint(&self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }
}

/// Result of a raycast hit (matches Unity's `RaycastHit`).
#[derive(Debug, Clone)]
pub struct RaycastHit {
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
    pub collider: Option<crate::world::GameObjectHandle>,
    pub rigidbody: Option<crate::world::GameObjectHandle>,
    pub transform: Option<crate::world::GameObjectHandle>,
    pub triangle_index: Option<u32>,
    pub texture_coord: Option<Vec2>,
}

/// 2D vector for texture coordinates etc.
#[derive(Debug, Clone, Copy, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Layer mask for physics queries (matches Unity's `LayerMask`).
#[derive(Debug, Clone, Copy, Default)]
pub struct LayerMask(pub i32);

impl LayerMask {
    pub fn NameToLayer(name: &str) -> i32 {
        match name {
            "Default" => 0,
            "TransparentFX" => 1,
            "Ignore Raycast" => 2,
            "Water" => 4,
            "UI" => 5,
            _ => -1,
        }
    }

    pub fn LayerToName(layer: i32) -> &'static str {
        match layer {
            0 => "Default",
            1 => "TransparentFX",
            2 => "Ignore Raycast",
            4 => "Water",
            5 => "UI",
            _ => "",
        }
    }

    pub fn GetMask(names: &[&str]) -> i32 {
        let mut mask = 0i32;
        for name in names {
            let layer = Self::NameToLayer(name);
            if layer >= 0 {
                mask |= 1 << layer;
            }
        }
        mask
    }
}

impl std::ops::BitOr for LayerMask {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        LayerMask(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for LayerMask {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        LayerMask(self.0 & rhs.0)
    }
}
```

- [ ] **Step 2: 在 lib.rs 中导出模块**

在 `crates/engine-core/src/lib.rs` 中添加：
```rust
pub mod raycast;
```

- [ ] **Step 3: 编写测试**

```rust
#[test]
fn test_ray_creation() {
    use engine_core::raycast::Ray;
    use engine_math::Vec3;
    let ray = Ray::new(Vec3::ZERO, Vec3::Y);
    assert_eq!(ray.origin, Vec3::ZERO);
    assert!((ray.direction.length() - 1.0).abs() < 0.001);
}

#[test]
fn test_ray_get_point() {
    use engine_core::raycast::Ray;
    use engine_math::Vec3;
    let ray = Ray::new(Vec3::ZERO, Vec3::X);
    let point = ray.GetPoint(5.0);
    assert!((point.x - 5.0).abs() < 0.001);
}

#[test]
fn test_layer_mask() {
    use engine_core::raycast::LayerMask;
    assert_eq!(LayerMask::NameToLayer("Default"), 0);
    assert_eq!(LayerMask::LayerToName(0), "Default");
    let mask = LayerMask::GetMask(&["Default", "UI"]);
    assert_eq!(mask, 1 | (1 << 5));
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p engine-core --test unity_api_completion_tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/engine-core/src/raycast.rs crates/engine-core/src/lib.rs crates/engine-core/tests/unity_api_completion_tests.rs
git commit -m "feat: add Ray, RaycastHit, LayerMask types"
```

---

### Task 2: Bounds

**Files:**
- Create: `crates/engine-core/src/bounds.rs`
- Modify: `crates/engine-core/src/lib.rs`
- Test: `crates/engine-core/tests/unity_api_completion_tests.rs`

- [ ] **Step 1: 创建 bounds.rs**

```rust
//! Axis-aligned bounding box (matches Unity's Bounds, BoundsInt).

use engine_math::Vec3;

/// Axis-aligned bounding box (matches Unity's `Bounds`).
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub center: Vec3,
    pub extents: Vec3,
}

impl Bounds {
    pub fn new(center: Vec3, size: Vec3) -> Self {
        Self { center, extents: size * 0.5 }
    }

    pub fn size(&self) -> Vec3 {
        self.extents * 2.0
    }

    pub fn min(&self) -> Vec3 {
        self.center - self.extents
    }

    pub fn max(&self) -> Vec3 {
        self.center + self.extents
    }

    pub fn Contains(&self, point: Vec3) -> bool {
        let min = self.min();
        let max = self.max();
        point.x >= min.x && point.x <= max.x
            && point.y >= min.y && point.y <= max.y
            && point.z >= min.z && point.z <= max.z
    }

    pub fn Intersects(&self, other: &Bounds) -> bool {
        let a_min = self.min();
        let a_max = self.max();
        let b_min = other.min();
        let b_max = other.max();
        a_min.x <= b_max.x && a_max.x >= b_min.x
            && a_min.y <= b_max.y && a_max.y >= b_min.y
            && a_min.z <= b_max.z && a_max.z >= b_min.z
    }

    pub fn Encapsulate(&mut self, point: Vec3) {
        let min = self.min();
        let max = self.max();
        let new_min = Vec3::new(
            min.x.min(point.x),
            min.y.min(point.y),
            min.z.min(point.z),
        );
        let new_max = Vec3::new(
            max.x.max(point.x),
            max.y.max(point.y),
            max.z.max(point.z),
        );
        self.center = (new_min + new_max) * 0.5;
        self.extents = (new_max - new_min) * 0.5;
    }

    pub fn EncapsulateBounds(&mut self, other: &Bounds) {
        self.Encapsulate(other.min());
        self.Encapsulate(other.max());
    }

    pub fn ClosestPoint(&self, point: Vec3) -> Vec3 {
        let min = self.min();
        let max = self.max();
        Vec3::new(
            point.x.clamp(min.x, max.x),
            point.y.clamp(min.y, max.y),
            point.z.clamp(min.z, max.z),
        )
    }

    pub fn SqrDistance(&self, point: Vec3) -> f32 {
        let closest = self.ClosestPoint(point);
        (closest - point).length_squared()
    }
}

impl Default for Bounds {
    fn default() -> Self {
        Self { center: Vec3::ZERO, extents: Vec3::ZERO }
    }
}
```

- [ ] **Step 2: 在 lib.rs 中导出**

```rust
pub mod bounds;
```

- [ ] **Step 3: 编写测试**

```rust
#[test]
fn test_bounds_contains() {
    use engine_core::bounds::Bounds;
    use engine_math::Vec3;
    let b = Bounds::new(Vec3::ZERO, Vec3::ONE);
    assert!(b.Contains(Vec3::ZERO));
    assert!(b.Contains(Vec3::new(0.4, 0.4, 0.4)));
    assert!(!b.Contains(Vec3::new(1.0, 1.0, 1.0)));
}

#[test]
fn test_bounds_intersects() {
    use engine_core::bounds::Bounds;
    use engine_math::Vec3;
    let a = Bounds::new(Vec3::ZERO, Vec3::ONE);
    let b = Bounds::new(Vec3::new(0.5, 0.0, 0.0), Vec3::ONE);
    let c = Bounds::new(Vec3::new(2.0, 0.0, 0.0), Vec3::ONE);
    assert!(a.Intersects(&b));
    assert!(!a.Intersects(&c));
}

#[test]
fn test_bounds_encapsulate() {
    use engine_core::bounds::Bounds;
    use engine_math::Vec3;
    let mut b = Bounds::new(Vec3::ZERO, Vec3::ONE);
    b.Encapsulate(Vec3::new(2.0, 2.0, 2.0));
    assert!(b.Contains(Vec3::new(2.0, 2.0, 2.0)));
}
```

- [ ] **Step 4: 运行测试确认通过**

- [ ] **Step 5: Commit**

```bash
git add crates/engine-core/src/bounds.rs crates/engine-core/src/lib.rs crates/engine-core/tests/unity_api_completion_tests.rs
git commit -m "feat: add Bounds type with Contains, Intersects, Encapsulate"
```

---

### Task 3: Mathf 结构体

**Files:**
- Create: `crates/engine-core/src/mathf.rs`
- Modify: `crates/engine-core/src/lib.rs`
- Test: `crates/engine-core/tests/unity_api_completion_tests.rs`

- [ ] **Step 1: 创建 mathf.rs**

```rust
//! Mathf utility struct (matches Unity's Mathf).

use std::f32::consts::PI;

/// Mathf constants and methods (matches Unity's `Mathf`).
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
        if delta > 180.0 { delta - 360.0 } else { delta };
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
        let omega = 2.0 / smooth_time.max(0.0001);
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
        let mut v = value.max(1);
        v -= 1;
        v |= v >> 1;
        v |= v >> 2;
        v |= v >> 4;
        v |= v >> 8;
        v |= v >> 16;
        v + 1
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
```

- [ ] **Step 2: 在 lib.rs 中导出**

```rust
pub mod mathf;
```

- [ ] **Step 3: 编写测试**

```rust
#[test]
fn test_mathf_lerp() {
    use engine_core::mathf::Mathf;
    assert!((Mathf.Lerp(0.0, 10.0, 0.5) - 5.0).abs() < 0.001);
    assert!((Mathf.Lerp(0.0, 10.0, 0.0) - 0.0).abs() < 0.001);
    assert!((Mathf.Lerp(0.0, 10.0, 1.0) - 10.0).abs() < 0.001);
}

#[test]
fn test_mathf_inverse_lerp() {
    use engine_core::mathf::Mathf;
    assert!((Mathf.InverseLerp(0.0, 10.0, 5.0) - 0.5).abs() < 0.001);
    assert!((Mathf.InverseLerp(0.0, 10.0, -1.0)).abs() < 0.001);
    assert!((Mathf.InverseLerp(0.0, 10.0, 11.0) - 1.0).abs() < 0.001);
}

#[test]
fn test_mathf_approximately() {
    use engine_core::mathf::Mathf;
    assert!(Mathf.Approximately(1.0, 1.0));
    assert!(Mathf.Approximately(1.0, 1.0000001));
    assert!(!Mathf.Approximately(1.0, 2.0));
}

#[test]
fn test_mathf_power_of_two() {
    use engine_core::mathf::Mathf;
    assert!(Mathf.IsPowerOfTwo(4));
    assert!(!Mathf.IsPowerOfTwo(5));
    assert_eq!(Mathf.NextPowerOfTwo(5), 8);
    assert_eq!(Mathf.ClosestPowerOfTwo(5), 4);
}
```

- [ ] **Step 4: 运行测试确认通过**

- [ ] **Step 5: Commit**

```bash
git add crates/engine-core/src/mathf.rs crates/engine-core/src/lib.rs crates/engine-core/tests/unity_api_completion_tests.rs
git commit -m "feat: add Mathf struct with Lerp, SmoothDamp, and utility methods"
```

---

## Phase 2: 物理增强

### Task 4: ForceMode 和 Rigidbody 增强

**Files:**
- Modify: `crates/engine-core/src/components.rs`
- Test: `crates/engine-core/tests/unity_api_completion_tests.rs`

- [ ] **Step 1: 添加 ForceMode 枚举和 Rigidbody 方法**

在 `components.rs` 中 Rigidbody 部分添加：

```rust
/// Force application mode (matches Unity's `ForceMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForceMode {
    Force,
    Impulse,
    VelocityChange,
    Acceleration,
}

impl Default for ForceMode {
    fn default() -> Self { Self::Force }
}

impl Rigidbody {
    pub fn AddForceWithMode(&mut self, force: Vec3, mode: ForceMode) {
        match mode {
            ForceMode::Force => self.velocity += force / self.mass,
            ForceMode::Impulse => self.velocity += force / self.mass,
            ForceMode::VelocityChange => self.velocity += force,
            ForceMode::Acceleration => self.velocity += force,
        }
    }

    pub fn AddTorqueWithMode(&mut self, torque: Vec3, mode: ForceMode) {
        match mode {
            ForceMode::Force => self.angular_velocity += torque / self.mass,
            ForceMode::Impulse => self.angular_velocity += torque / self.mass,
            ForceMode::VelocityChange => self.angular_velocity += torque,
            ForceMode::Acceleration => self.angular_velocity += torque,
        }
    }

    pub fn AddForceAtPosition(&mut self, force: Vec3, position: Vec3, center_of_mass: Vec3) {
        self.velocity += force / self.mass;
        let torque = (position - center_of_mass).cross(force);
        self.angular_velocity += torque / self.mass;
    }

    pub fn AddRelativeForce(&mut self, force: Vec3, rotation: Quat) {
        let world_force = rotation * force;
        self.velocity += world_force / self.mass;
    }

    pub fn AddRelativeTorque(&mut self, torque: Vec3, rotation: Quat) {
        let world_torque = rotation * torque;
        self.angular_velocity += world_torque / self.mass;
    }

    pub fn WakeUp(&mut self) {
        // Wake up is implicit when velocity is set
    }

    pub fn IsSleeping(&self) -> bool {
        self.velocity.length_squared() < 0.001 && self.angular_velocity.length_squared() < 0.001
    }
}
```

- [ ] **Step 2: 编写测试**

```rust
#[test]
fn test_rigidbody_force_modes() {
    use engine_core::components::{Rigidbody, ForceMode};
    use engine_math::Vec3;
    let mut rb = Rigidbody::default();
    rb.mass = 2.0;
    rb.AddForceWithMode(Vec3::new(10.0, 0.0, 0.0), ForceMode::Force);
    assert!((rb.velocity.x - 5.0).abs() < 0.001);
}

#[test]
fn test_rigidbody_impulse() {
    use engine_core::components::{Rigidbody, ForceMode};
    use engine_math::Vec3;
    let mut rb = Rigidbody::default();
    rb.mass = 2.0;
    rb.AddForceWithMode(Vec3::new(10.0, 0.0, 0.0), ForceMode::VelocityChange);
    assert!((rb.velocity.x - 10.0).abs() < 0.001);
}
```

- [ ] **Step 3: 运行测试确认通过**

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/components.rs crates/engine-core/tests/unity_api_completion_tests.rs
git commit -m "feat: add ForceMode and enhanced Rigidbody methods"
```

---

### Task 5: Bounds 补全 Transform 方法

**Files:**
- Modify: `crates/engine-core/src/transform.rs`
- Test: `crates/engine-core/tests/unity_api_completion_tests.rs`

- [ ] **Step 1: 添加 Transform 缺失方法**

在 `transform.rs` 中添加：

```rust
/// Set local position and rotation and scale (matches Unity's Transform.SetLocalPositionAndRotationAndScale).
pub fn SetLocalPositionAndRotationAndScale(
    &mut self,
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
) {
    self.local_position = position;
    self.local_rotation = rotation;
    self.local_scale = scale;
}

/// Get the total number of children (matches Unity's Transform.childCount).
pub fn ChildCount(&self) -> usize {
    self.children.len()
}

/// Batch transform points (matches Unity's Transform.TransformPoints).
pub fn TransformPoints(&self, points: &[Vec3]) -> Vec<Vec3> {
    points.iter().map(|&p| self.TransformPoint(p)).collect()
}

/// Batch inverse transform points (matches Unity's Transform.InverseTransformPoints).
pub fn InverseTransformPoints(&self, points: &[Vec3]) -> Vec<Vec3> {
    points.iter().map(|&p| self.InverseTransformPoint(p)).collect()
}

/// Batch transform directions (matches Unity's Transform.TransformDirections).
pub fn TransformDirections(&self, directions: &[Vec3]) -> Vec<Vec3> {
    directions.iter().map(|&d| self.TransformDirection(d)).collect()
}

/// Batch inverse transform directions (matches Unity's Transform.InverseTransformDirections).
pub fn InverseTransformDirections(&self, directions: &[Vec3]) -> Vec<Vec3> {
    directions.iter().map(|&d| self.InverseTransformDirection(d)).collect()
}
```

- [ ] **Step 2: 编写测试**

```rust
#[test]
fn test_transform_batch_methods() {
    use engine_core::transform::Transform;
    use engine_math::{Vec3, Quat};
    let mut t = Transform::default();
    t.SetLocalPosition(Vec3::new(1.0, 0.0, 0.0));
    let points = vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)];
    let transformed = t.TransformPoints(&points);
    assert_eq!(transformed.len(), 2);
    assert!((transformed[0].x - 1.0).abs() < 0.001);
}
```

- [ ] **Step 3: 运行测试确认通过**

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/transform.rs crates/engine-core/tests/unity_api_completion_tests.rs
git commit -m "feat: add batch Transform methods and SetLocalPositionAndRotationAndScale"
```

---

## Phase 3: MonoBehaviour 生命周期补全

### Task 6: 补全 MonoBehaviour 回调

**Files:**
- Modify: `crates/engine-core/src/monobehaviour.rs`
- Test: `crates/engine-core/tests/unity_api_completion_tests.rs`

- [ ] **Step 1: 添加缺失的生命周期回调**

在 `monobehaviour.rs` 的 `MonoBehaviour` trait 中添加：

```rust
/// Called by the editor to validate and reload the inspector (matches MonoBehaviour.OnValidate).
fn OnValidate(&mut self) {}

/// Called when the script is loaded or a value changes in the inspector (matches MonoBehaviour.Reset).
fn Reset(&mut self) {}

/// Called when the transform parent changes (matches MonoBehaviour.OnTransformParentChanged).
fn OnTransformParentChanged(&mut self) {}

/// Called after all transform children have changed (matches MonoBehaviour.OnTransformChildrenChanged).
fn OnTransformChildrenChanged(&mut self) {}

/// Called when a joint attached to the same game object broke (matches MonoBehaviour.OnJointBreak).
fn OnJointBreak(&mut self, _breakForce: f32) {}

/// Called during the render loop (matches MonoBehaviour.OnRenderObject).
fn OnRenderObject(&mut self) {}

/// Called before any camera renders the object (matches MonoBehaviour.OnWillRenderObject).
fn OnWillRenderObject(&mut self) {}

/// Called before a camera renders the scene (matches MonoBehaviour.OnPreRender).
fn OnPreRender(&mut self) {}

/// Called after a camera renders the scene (matches MonoBehaviour.OnPostRender).
fn OnPostRender(&mut self) {}
```

- [ ] **Step 2: 编写测试**

```rust
#[test]
fn test_monobehaviour_new_callbacks() {
    use engine_core::monobehaviour::MonoBehaviour;
    use engine_core::context::Context;

    struct TestBehaviour {
        validated: bool,
    }

    impl MonoBehaviour for TestBehaviour {
        fn as_any(&self) -> &dyn std::any::Any { self }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
        fn TypeName(&self) -> &str { "TestBehaviour" }

        fn OnValidate(&mut self) {
            self.validated = true;
        }
    }

    let mut tb = TestBehaviour { validated: false };
    tb.OnValidate();
    assert!(tb.validated);
}
```

- [ ] **Step 3: 运行测试确认通过**

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/monobehaviour.rs crates/engine-core/tests/unity_api_completion_tests.rs
git commit -m "feat: add missing MonoBehaviour lifecycle callbacks"
```

---

## Phase 4: 缺失组件

### Task 7: CharacterController 组件

**Files:**
- Create: `crates/engine-core/src/character_controller.rs`
- Modify: `crates/engine-core/src/lib.rs`, `crates/engine-core/src/components.rs`
- Test: `crates/engine-core/tests/unity_api_completion_tests.rs`

- [ ] **Step 1: 创建 character_controller.rs**

```rust
//! Character controller component (matches Unity's CharacterController).

use crate::component::Component;
use engine_math::Vec3;
use std::any::Any;

/// Character controller component (matches Unity's `CharacterController`).
#[derive(Debug, Clone)]
pub struct CharacterController {
    pub slope_limit: f32,
    pub step_offset: f32,
    pub skin_width: f32,
    pub min_move_distance: f32,
    pub center: Vec3,
    pub radius: f32,
    pub height: f32,
    pub is_grounded: bool,
    pub velocity: Vec3,
}

impl Default for CharacterController {
    fn default() -> Self {
        Self {
            slope_limit: 45.0,
            step_offset: 0.3,
            skin_width: 0.08,
            min_move_distance: 0.001,
            center: Vec3::new(0.0, 1.0, 0.0),
            radius: 0.5,
            height: 2.0,
            is_grounded: false,
            velocity: Vec3::ZERO,
        }
    }
}

impl Component for CharacterController {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

impl CharacterController {
    /// Move the character (matches CharacterController.Move).
    pub fn Move(&mut self, motion: Vec3) -> CollisionFlags {
        self.velocity = motion;
        // Simplified: just report no collision
        CollisionFlags::NONE
    }

    /// Simple move (matches CharacterController.SimpleMove).
    pub fn SimpleMove(&mut self, speed: f32) {
        // Simplified: apply speed to velocity
        if speed > 0.0 {
            self.velocity = Vec3::new(0.0, 0.0, -speed);
        }
    }

    /// Check if the character is grounded (matches CharacterController.isGrounded).
    pub fn IsGrounded(&self) -> bool {
        self.is_grounded
    }
}

/// Collision flags for CharacterController.Move (matches Unity's CollisionFlags).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionFlags(i32);

impl CollisionFlags {
    pub const NONE: Self = Self(0);
    pub const SIDE: Self = Self(1);
    pub const ABOVE: Self = Self(2);
    pub const BELOW: Self = Self(4);
}

impl std::ops::BitOr for CollisionFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        CollisionFlags(self.0 | rhs.0)
    }
}
```

- [ ] **Step 2: 在 lib.rs 和 components.rs 中导出**

在 `lib.rs` 中添加：
```rust
pub mod character_controller;
```

在 `components.rs` 末尾添加：
```rust
pub use crate::character_controller::CharacterController;
```

- [ ] **Step 3: 编写测试**

```rust
#[test]
fn test_character_controller() {
    use engine_core::character_controller::CharacterController;
    use engine_math::Vec3;
    let mut cc = CharacterController::default();
    assert!(!cc.IsGrounded());
    cc.is_grounded = true;
    assert!(cc.IsGrounded());
    cc.SimpleMove(5.0);
    assert!((cc.velocity.z + 5.0).abs() < 0.001);
}
```

- [ ] **Step 4: 运行测试确认通过**

- [ ] **Step 5: Commit**

```bash
git add crates/engine-core/src/character_controller.rs crates/engine-core/src/lib.rs crates/engine-core/src/components.rs crates/engine-core/tests/unity_api_completion_tests.rs
git commit -m "feat: add CharacterController component"
```

---

### Task 8: MeshFilter 组件

**Files:**
- Modify: `crates/engine-core/src/components.rs`
- Test: `crates/engine-core/tests/unity_api_completion_tests.rs`

- [ ] **Step 1: 在 components.rs 中添加 MeshFilter**

```rust
// ============================================================
// MeshFilter (Unity: UnityEngine.MeshFilter)
// ============================================================

/// Mesh filter component (matches Unity's `MeshFilter`).
#[derive(Debug, Clone)]
pub struct MeshFilter {
    /// The mesh used by this filter (matches `MeshFilter.mesh`).
    pub mesh: String,
}

impl Default for MeshFilter {
    fn default() -> Self {
        Self { mesh: "Cube".to_string() }
    }
}

impl Component for MeshFilter {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}
```

- [ ] **Step 2: 编写测试**

```rust
#[test]
fn test_mesh_filter() {
    use engine_core::components::MeshFilter;
    let mf = MeshFilter::default();
    assert_eq!(mf.mesh, "Cube");
}
```

- [ ] **Step 3: 运行测试确认通过**

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/components.rs crates/engine-core/tests/unity_api_completion_tests.rs
git commit -m "feat: add MeshFilter component"
```

---

## Phase 5: 工具类

### Task 9: Random 工具类

**Files:**
- Create: `crates/engine-core/src/random.rs`
- Modify: `crates/engine-core/src/lib.rs`
- Test: `crates/engine-core/tests/unity_api_completion_tests.rs`

- [ ] **Step 1: 创建 random.rs**

```rust
//! Random utility (matches Unity's Random).

use engine_math::{Vec3, Quat};
use engine_math::Vec4;

/// Random number generator (matches Unity's `Random`).
pub struct Random;

impl Random {
    /// Returns a random float in range [min, max] (matches `Random.Range(float, float)`).
    pub fn Range(min: f32, max: f32) -> f32 {
        let r = rand::random::<f32>();
        min + r * (max - min)
    }

    /// Returns a random integer in range [min, max] (inclusive) (matches `Random.Range(int, int)`).
    pub fn RangeInt(min: i32, max: i32) -> i32 {
        let r = rand::random::<u32>() as i32;
        min + (r.abs() % (max - min + 1))
    }

    /// Returns a random point inside a sphere (matches `Random.insideUnitSphere`).
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

    /// Returns a point on the surface of a unit sphere (matches `Random.onUnitSphere`).
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

    /// Returns a random point inside a unit circle (matches `Random.insideUnitCircle`).
    pub fn InsideUnitCircle() -> engine_core::raycast::Vec2 {
        loop {
            let v = engine_core::raycast::Vec2::new(
                Self::Range(-1.0, 1.0),
                Self::Range(-1.0, 1.0),
            );
            if v.x * v.x + v.y * v.y <= 1.0 {
                return v;
            }
        }
    }

    /// Returns a random rotation (matches `Random.rotation`).
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

    /// Returns a random value between 0.0 and 1.0 (matches `Random.value`).
    pub fn Value() -> f32 {
        rand::random::<f32>()
    }
}
```

- [ ] **Step 2: 在 lib.rs 中导出**

```rust
pub mod random;
```

- [ ] **Step 3: 编写测试**

```rust
#[test]
fn test_random_range() {
    use engine_core::random::Random;
    for _ in 0..100 {
        let v = Random::Range(2.0, 5.0);
        assert!(v >= 2.0 && v <= 5.0);
    }
}

#[test]
fn test_random_inside_sphere() {
    use engine_core::random::Random;
    for _ in 0..100 {
        let v = Random::InsideUnitSphere();
        assert!(v.length() <= 1.001);
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

- [ ] **Step 5: Commit**

```bash
git add crates/engine-core/src/random.rs crates/engine-core/src/lib.rs crates/engine-core/tests/unity_api_completion_tests.rs
git commit -m "feat: add Random utility class"
```

---

### Task 10: Debug 工具类

**Files:**
- Create: `crates/engine-core/src/debug_utils.rs`
- Modify: `crates/engine-core/src/lib.rs`
- Test: `crates/engine-core/tests/unity_api_completion_tests.rs`

- [ ] **Step 1: 创建 debug_utils.rs**

```rust
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

    pub fn LogFormat(format: &str, args: &[&dyn std::fmt::Display]) {
        let mut msg = format.to_string();
        for (i, arg) in args.iter().enumerate() {
            msg = msg.replace(&format!("{{{}}}", i), &arg.to_string());
        }
        println!("[LOG] {}", msg);
    }

    pub fn DrawRay(from: Vec3, direction: Vec3, color: [f32; 4], duration: f32) {
        // Placeholder: would render a debug ray
        let _ = (from, direction, color, duration);
    }

    pub fn DrawLine(start: Vec3, end: Vec3, color: [f32; 4], duration: f32) {
        // Placeholder: would render a debug line
        let _ = (start, end, color, duration);
    }
}

/// Gizmos drawing (matches Unity's `Gizmos`).
pub struct Gizmos;

impl Gizmos {
    pub static mut COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

    pub fn Color(color: [f32; 4]) {
        unsafe { Self::COLOR = color; }
    }

    pub fn DrawSphere(center: Vec3, radius: f32) {
        // Placeholder
        let _ = (center, radius);
    }

    pub fn DrawCube(center: Vec3, size: Vec3) {
        // Placeholder
        let _ = (center, size);
    }

    pub fn DrawWireSphere(center: Vec3, radius: f32) {
        // Placeholder
        let _ = (center, radius);
    }

    pub fn DrawWireCube(center: Vec3, size: Vec3) {
        // Placeholder
        let _ = (center, size);
    }

    pub fn DrawLine(from: Vec3, to: Vec3) {
        // Placeholder
        let _ = (from, to);
    }

    pub fn DrawRay(from: Vec3, direction: Vec3) {
        // Placeholder
        let _ = (from, direction);
    }
}
```

- [ ] **Step 2: 在 lib.rs 中导出**

```rust
pub mod debug_utils;
```

- [ ] **Step 3: 编写测试**

```rust
#[test]
fn test_debug_log() {
    use engine_core::debug_utils::Debug;
    Debug.Log("test message");
    Debug.LogWarning("test warning");
    Debug.LogError("test error");
}
```

- [ ] **Step 4: 运行测试确认通过**

- [ ] **Step 5: Commit**

```bash
git add crates/engine-core/src/debug_utils.rs crates/engine-core/src/lib.rs crates/engine-core/tests/unity_api_completion_tests.rs
git commit -m "feat: add Debug and Gizmos utility classes"
```

---

## Phase 6: 场景管理

### Task 11: SceneManager

**Files:**
- Create: `crates/engine-core/src/scene_management.rs`
- Modify: `crates/engine-core/src/lib.rs`
- Test: `crates/engine-core/tests/unity_api_completion_tests.rs`

- [ ] **Step 1: 创建 scene_management.rs**

```rust
//! Scene management (matches Unity's SceneManager).

/// Scene handle (matches Unity's Scene).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneHandle(pub u32);

impl SceneHandle {
    pub const INVALID: Self = Self(u32::MAX);
}

/// Scene information.
#[derive(Debug, Clone)]
pub struct SceneInfo {
    pub name: String,
    pub path: String,
    pub handle: SceneHandle,
    pub is_loaded: bool,
    pub root_count: usize,
}

/// Scene manager (matches Unity's `SceneManager`).
pub struct SceneManager {
    scenes: Vec<SceneInfo>,
    active_scene: Option<SceneHandle>,
}

impl Default for SceneManager {
    fn default() -> Self {
        Self {
            scenes: Vec::new(),
            active_scene: None,
        }
    }
}

impl SceneManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the active scene (matches `SceneManager.GetActiveScene`).
    pub fn GetActiveScene(&self) -> Option<&SceneInfo> {
        self.active_scene.and_then(|h| self.scenes.iter().find(|s| s.handle == h))
    }

    /// Load a scene by name (matches `SceneManager.LoadScene`).
    pub fn LoadScene(&mut self, name: &str) -> Result<SceneHandle, String> {
        let handle = SceneHandle(self.scenes.len() as u32);
        self.scenes.push(SceneInfo {
            name: name.to_string(),
            path: format!("Scenes/{}.unity", name),
            handle,
            is_loaded: true,
            root_count: 0,
        });
        self.active_scene = Some(handle);
        Ok(handle)
    }

    /// Unload a scene (matches `SceneManager.UnloadScene`).
    pub fn UnloadScene(&mut self, handle: SceneHandle) -> Result<(), String> {
        if let Some(scene) = self.scenes.iter_mut().find(|s| s.handle == handle) {
            scene.is_loaded = false;
            Ok(())
        } else {
            Err("Scene not found".to_string())
        }
    }

    /// Get scene count (matches `SceneManager.sceneCount`).
    pub fn SceneCount(&self) -> usize {
        self.scenes.len()
    }

    /// Get all loaded scenes (matches `SceneManager.GetLoadedScenes`).
    pub fn GetLoadedScenes(&self) -> Vec<&SceneInfo> {
        self.scenes.iter().filter(|s| s.is_loaded).collect()
    }
}
```

- [ ] **Step 2: 在 lib.rs 中导出**

```rust
pub mod scene_management;
```

- [ ] **Step 3: 编写测试**

```rust
#[test]
fn test_scene_manager() {
    use engine_core::scene_management::SceneManager;
    let mut sm = SceneManager::new();
    assert_eq!(sm.SceneCount(), 0);
    let handle = sm.LoadScene("TestScene").unwrap();
    assert_eq!(sm.SceneCount(), 1);
    let active = sm.GetActiveScene().unwrap();
    assert_eq!(active.name, "TestScene");
    sm.UnloadScene(handle).unwrap();
    assert!(!sm.GetLoadedScenes().is_empty());
}
```

- [ ] **Step 4: 运行测试确认通过**

- [ ] **Step 5: Commit**

```bash
git add crates/engine-core/src/scene_management.rs crates/engine-core/src/lib.rs crates/engine-core/tests/unity_api_completion_tests.rs
git commit -m "feat: add SceneManager for scene loading/unloading"
```

---

## 执行顺序

| 阶段 | Task | 依赖 | 预计时间 |
|------|------|------|----------|
| Phase 1 | Task 1-3 | 无 | 1小时 |
| Phase 2 | Task 4-5 | Task 1 | 30分钟 |
| Phase 3 | Task 6 | 无 | 20分钟 |
| Phase 4 | Task 7-8 | 无 | 30分钟 |
| Phase 5 | Task 9-10 | Task 1 | 30分钟 |
| Phase 6 | Task 11 | 无 | 20分钟 |

**总计约 3 小时**

---

## 后续可扩展

完成以上计划后，可继续补充：
- Animator / Animation 系统
- Canvas / UI 系统
- Input 系统
- MeshCollider / Joint
- SkinnedMeshRenderer
- ReflectionProbe / LODGroup
