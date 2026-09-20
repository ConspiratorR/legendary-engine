---
feature: phase26-broadphase-world-aabb
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 56f8fee..HEAD
---

# Phase 26 — Broadphase 旋转感知世界 AABB

## Report

**What was built** — Broadphase now expands collider AABBs by body rotation. New `rotated_aabb_half_extents(rot, local_half)` computes the world AABB of a rotated local OBB (`e_i = Σ |R_ij| h_j`). Insert path uses `center = pos + rot * offset` and `half = rotated_aabb_half_extents(rot, shape.half_extents()) + |rot*offset|`. Unit test: identity unchanged; 90° Z swaps x/y; 45° long box expands both axes.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-physics --lib` | PASS **81** |
| `cargo test -p engine-physics --test physics_tests` | PASS |
| `cargo test -p engine-core --lib` | PASS 261 |
| `cargo test -p engine-editor --test editor_tests` | PASS 62 |

**Journey log** —
1. Local-axis half_extents + rotation without expansion caused missed narrow-phase pairs.
2. Formula uses rotated basis vectors’ absolute component sums — standard OBB→AABB.
3. Offset is also rotated into world before AABB add.
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
