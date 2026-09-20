---
feature: phase25-oriented-capsule
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: cf403b5..HEAD
---

# Phase 25 — Oriented Capsule 轴向支持

## Report

**What was built** — `CapsuleAxis` {X,Y,Z} added; `ColliderShape::Capsule` carries `axis`; `capsule_segment` / broadphase `half_extents()` are axis-aware; Unity `CapsuleCollider.direction` maps via `CapsuleAxis::from_unity_direction` in runtime and editor bridges (Y-only warn removed). `Collider::capsule_with_axis` constructor added; existing `capsule()` defaults to Y. Unit test covers X-axis half-extents and bridge axis mapping.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-physics --lib` | PASS **80** |
| `cargo test -p engine-physics --test physics_tests` | PASS |
| `cargo test -p engine-core --lib` | PASS 261 |
| `cargo test -p engine-editor --test editor_tests` | PASS 62 |

**Journey log** —
1. Capsule `axis` field required updating all shape match arms + check_* APIs.
2. Broadphase uses `ColliderShape::half_extents()` so axis is consistent.
3. Bridge no longer warns on non-Y direction — it maps the axis.
4. git merge/push not handled per user preference.

## [S1] Problem
Capsule fixed Y-axis; Unity direction ignored.

## [S2] Design
CapsuleAxis + axis-aware segment/broadphase/bridge as specified.

## [S3] Out of Scope
Full material/contact manifold, WASM browser, git merge/push

## Tasks

- [x] T1: CapsuleAxis + segment + broadphase (covers: S2)
- [x] T2: Bridge direction→axis (covers: S2; depends: T1)
- [x] T3: 测试 + docs + finalize (covers: S2)
