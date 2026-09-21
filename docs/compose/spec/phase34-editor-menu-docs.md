---
feature: phase34-editor-menu-docs
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: aefe06c..HEAD
---

# Phase 34 — 移除菜单对齐 + 分支文档 12–34

## Report

**What was built** — Editor remove-component menu gains granular **盒/球/胶囊碰撞体** entries (aligned with add menu keys `box_collider`/`sphere_collider`/`capsule_collider`). Aggregate **物理** still removes Rigidbody + all colliders. Docs: multi-collider priority **Sphere → Box → Capsule** (bridge/serializer/inspector label) and remove-menu semantics in `unity-storage-animation.md`; BRANCH_INDEX 12–34 + README 阶段 34.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-editor --test editor_tests` | PASS **63** |
| `cargo test -p engine-core --lib` | PASS 261 |

**Journey log** —
1. Remove-menu `comp_types` zip list must stay aligned with exists-tuple count.
2. Multi-collider: only priority shape enters unity_bridge sim — document for authors.
3. git merge/push not handled per user preference.

## [S1] Problem
Remove menu lacked granular collider entries; multi-collider behavior undocumented.

## [S2] Design
Granular remove + docs; aggregate physics remove unchanged.

## [S3] Out of Scope
Multi-collider physics, WASM browser, git merge/push

## Tasks

- [x] T1: 移除菜单对齐 (covers: S2)
- [x] T2: 文档 + 门禁 + finalize (covers: S2; depends: T1)
