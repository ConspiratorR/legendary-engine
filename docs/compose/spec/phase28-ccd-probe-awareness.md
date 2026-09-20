---
feature: phase28-ccd-probe-awareness
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 18cf061..HEAD
---

# Phase 28 — CCD 探针 offset/形状感知

## Report

**What was built** — CCD sweeps the **probe world center** (`pos + rot*offset`) when integrating fast bodies, then converts the safe center back to body origin before `SetPosition`. Probe radius uses exact `Sphere.radius` when applicable; other shapes keep bounding-sphere. e2e test: probe origin at y=-2 with offset +Y, static wall at y=0, high velocity — CCD clamps so world center stays near the wall instead of tunneling.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-physics --lib` | PASS |
| `cargo test -p engine-physics --test physics_tests` | PASS **66** |
| `cargo test -p engine-core --lib` | PASS 261 |
| `cargo test -p engine-editor --test editor_tests` | PASS 62 |

**Journey log** —
1. Probe origin ≠ collider center when `offset ≠ 0`; sweep must use world center.
2. Safe **center** cannot be written to `SetPosition` (origin) — subtract `R*offset`.
3. git merge/push not handled per user preference.

## [S1] Problem
CCD probe ignored offset/shape.

## [S2] Design
World-center sweep + origin writeback; sphere radius; e2e + docs.

## [S3] Out of Scope
Exact OBB/capsule probe sweep, WASM browser, git merge/push

## Tasks

- [x] T1: ccd_sweep 探针世界中心 (covers: S2)
- [x] T2: docs + finalize (covers: S2)
