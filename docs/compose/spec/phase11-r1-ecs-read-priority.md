---
feature: phase11-r1-ecs-read-priority
status: delivered
updated: 2026-07-13
branch: phase11-r1-read
commits: 149ff3d151e7e5604bdc112823b88014cbbcab9a..54e161613d401774a9cd7b7b36de45524796339e
---

# Phase 11 R1 — ECS Read Priority + Baseline Cleanup

## Report

**What was built** — Phase 11 first slice (S1 cleanup + S2 dual-read acceptance) on branch `phase11-r1-read`. Production dual-read preference (`GetTransform/Name/Tag/Active/Layer/Parent/Children` → ECS under `unity-world-primary`) was already on `main`; this slice locks the contract with a dual-mode divergence test (`test_dual_read_storage_mode_contract_when_ecs_diverges`) that mutates **only** ECS mirrors (including a decoy parent) and asserts public reads follow the compiled feature. Roadmap / PROJECT_SUMMARY / migration-guide / lifecycle docs were refreshed so dual-read is documented as implemented and still-deferred write-authority work points at the phase 11 plan, not completed P2.12 or unmerged p2 branches. Workspace default features remain `["audio"]`.

**Verification** —
- `cargo test -p engine-core --lib` — PASS (220)
- `cargo test -p engine-core --lib --features unity-world-primary` — PASS (240)
- `cargo test -p engine-core --test unity_lifecycle_tests` — PASS (21)
- `cargo test -p engine-core --test unity_lifecycle_tests --features unity-world-primary` — PASS (23)
- `cargo test -p engine-editor --test editor_tests` — PASS (58)
- `cargo build -p engine-editor` — PASS
- `cargo run -p engine-editor` smoke ~12s — PASS (process ran; wgpu Vulkan + RTX 5080 init)
- Dual-mode contract test re-run after GetParent decoy fix — PASS both feature states

**Journey log** —
1. Session policy blocked `git worktree add` and `git branch -d`; work proceeded on in-place branch `phase11-r1-read`. Merged-branch cleanup remains for user/orchestrator: `docs-unity-exit-criteria`, `p2-4-flag-integration`, `p2-4-storage-merge`, `p2-4-transform-dual-read`, `p2-6-editor-scene-path`, `p2-deferred-storage`, `p2-viewport-unity-source`, `unity-lifecycle-refactor`.
2. Dual-read production code was already complete — correct S2 scope is docs + acceptance tests, not API redesign.
3. Review found GetParent was not actually diverged in the first test draft (ECS parent equalled array parent); fixed with a decoy parent handle.
4. Review found roadmap still pointed write-authority at completed P2.12 and told users to merge already-merged `p2-viewport-unity-source`; both corrected.
5. Review found migration-guide had two dual-read tables (one incomplete); merged into a single complete table including `GetLayer` + feature-off note.
6. Focused re-review after fix: all four prior criticals confirmed FIXED at working tree `54e1616`.

## [S1] Problem

Phase 10 landed dual-write slices on `main`, but local feature branches remained, docs understated dual-read completion, and dual-mode feature-off array authority when ECS mirrors diverge was not locked by one explicit test.

## [S2] Design

### Storage contract (unchanged)

| Mode | Read API | Write authority |
|------|----------|-----------------|
| `unity-world-primary` **off** (default) | Array / `GameObject` fields | Array; ECS mirrors may exist but are **not** read |
| `unity-world-primary` **on** | ECS component when linked + present, else array | Array write-through or ECS-primary (`with_ecs_transform_mut`) |

### Dual-read inventory

| Public read | Feature-on preferred source | Fallback |
|-------------|----------------------------|----------|
| `GetTransform` | ECS `Transform` | `GetTransformArray` |
| `GetName` | ECS `GameObjectName` | `gameobject_data` |
| `GetTag` | ECS `GameObjectTag` | `gameobject_data` |
| `IsActive` | ECS `GameObjectActive` | `gameobject_data` |
| `GetLayer` | ECS `GameObjectLayer` | `gameobject_data` |
| `GetParent` | ECS `GameObjectParent` | `GetParentArray` |
| `GetChildren` | ECS `GameObjectChildren` | `GetChildrenArray` |

### Contracts for this slice

1. No production API redesign — existing dual-read preference stays; docs + dual-mode tests only.
2. Divergence test must pass under both feature modes; GetParent must point ECS at a **different** handle than array.
3. Workspace default features stay `["audio"]`.
4. Worktree/branch-delete overrides: session policy blocked both; branch `phase11-r1-read` in main checkout; cleanup deferred.
5. Editor smoke: tests + binary launch verified; interactive Play not automated this slice.

## [S3] Out of Scope

- S3 write-authority slices (Transform/Hierarchy/Identity/MB authority move)
- Enabling `unity-world-primary` by default
- engine-scene Transform removal / animation keyframe cut (S4)
- VR/AR, Android NDK, WASM SceneRuntime full
- Mouse DPI root-cause fix
- Remote `git push`
- Deleting merged local branches (policy-blocked for agent)

## Tasks

- [ ] T1: Delete merged local branches — **blocked** by session ref-store policy; leftover list in Report (covers: S1)
- [x] T2: Sync README / PROJECT_SUMMARY / migration-guide / roadmap dual-read status — dual-read implemented; still-deferred write-authority → phase 11 plan; P2.12 pointer removed (covers: S1)
- [x] T3: Add dual-mode divergence acceptance test — `test_dual_read_storage_mode_contract_when_ecs_diverges` green both modes; GetParent decoy divergence included (covers: S2)
- [x] T4: Commit plan + feature doc + code/docs on `phase11-r1-read` (covers: S1; S2)
- [x] T5: Run dual-mode verification gate — listed commands PASS (covers: S2)
- [x] T6: Editor smoke — editor_tests 58 PASS; engine-editor process launched with GPU init (covers: S2)
