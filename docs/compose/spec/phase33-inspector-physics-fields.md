---
feature: phase33-inspector-physics-fields
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 5393bc6..HEAD
---

# Phase 33 — Inspector 物理字段编辑

## Report

**What was built** — Editor Inspector physics section extended: Rigidbody **休眠** + **速度** vec3; Sphere radius/center/触发器; Box size/center/触发器; Capsule radius/height/center/axis (0/1/2)/触发器. Capsule axis uses a float slider rounded to i32. Test `physics_collider_fields_editable_on_world` asserts World component fields match Inspector data-path mutations (63 editor tests).

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-editor --test editor_tests` | PASS **63** |

**Journey log** —
1. Inspector previously read-only collider **name** only — no shape params.
2. Capsule `direction` is i32 — UI uses local f32 + round().
3. git merge/push not handled per user preference.

## [S1] Problem
Inspector missing collider/Sleep/velocity fields.

## [S2] Design
Editable UI fields + World mutation test.

## [S3] Out of Scope
Collision gizmos, WASM/Android, git merge/push

## Tasks

- [x] T1: Inspector 物理字段 UI (covers: S2)
- [x] T2: 文档 + finalize (covers: S2)
