---
feature: phase26-broadphase-world-aabb
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 56f8fee..HEAD
---

# Phase 26 — Broadphase 旋转感知世界 AABB

## Report

**What was built** — Broadphase expands collider AABBs by body rotation via `rotated_aabb_half_extents(rot, local_half)`. Insert path: `center = pos + rot * offset`, `half = rotated_aabb_half_extents(rot, shape.half_extents())` only — **offset is not double-counted** in half (C1 fix). Unit tests cover formula (identity/90°/45°) and AABB composition (offset center, tight half).

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-physics --lib` | PASS |
| `cargo test -p engine-physics --test physics_tests` | PASS |
| `cargo test -p engine-core --lib` | PASS 261 |
| `cargo test -p engine-editor --test editor_tests` | PASS 62 |

**Journey log** —
1. Local half_extents without rotation → missed narrow-phase pairs on elongated rotated bodies.
2. **C1**: `center += R*offset` and `half += |R*offset|` are mutually exclusive; keep center shift only.
3. CCD `sweep_sphere_aabb` still rotation-blind (residual, later phase).
4. git merge/push not handled per user preference.

## [S1] Problem
Broadphase ignored rotation → undersized AABBs.

## [S2] Design
`rotated_aabb_half_extents` + insert path as specified.

## [S3] Out of Scope
Exact OBB broadphase, WASM browser, git merge/push

## Tasks

- [x] T1: rotated_aabb_half_extents + broadphase 接入 (covers: S2)
- [x] T2: docs + finalize (covers: S2)
