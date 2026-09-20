---
feature: phase27-ccd-rotated-aabb
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 6f07fac..HEAD
---

# Phase 27 — CCD 旋转感知 AABB 扫描

## Report

**What was built** — `PhysicsWorld::ccd_sweep` builds obstacle AABBs with the same composition as phase26 broadphase: `center = pos + rot*offset`, `half = rotated_aabb_half_extents(rot, shape.half_extents())`. Box/Capsule/Cylinder all use this world AABB for `sweep_sphere_aabb` instead of unrotated local extents. Unit test asserts 90° Z long-box AABB swap and that a vertical sweep hits the rotated Y-long AABB.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-physics --lib` | PASS **83** |
| `cargo test -p engine-physics --test physics_tests` | PASS |
| `cargo test -p engine-core --lib` | PASS 261 |
| `cargo test -p engine-editor --test editor_tests` | PASS 62 |

**Journey log** —
1. Pre-phase27 CCD used `pos ± local half` — rotation-blind tunnel risk.
2. Sphere still uses bounding-sphere sweep; non-sphere shapes use rotated AABB.
3. git merge/push not handled per user preference.

## [S1] Problem
CCD ignored rotation/offset on obstacle AABBs.

## [S2] Design
Same rotated AABB composition as broadphase; tests + docs.

## [S3] Out of Scope
Exact OBB sweep, WASM browser, git merge/push

## Tasks

- [x] T1: ccd_sweep 使用旋转 AABB (covers: S2)
- [x] T2: docs + finalize (covers: S2)
