---
feature: phase11-r1-ecs-read-priority
status: in-progress
updated: 2026-07-13
branch: phase11-r1-read
commits: 
---

# Phase 11 R1 — ECS Read Priority + Baseline Cleanup

## Report

## [S1] Problem

Phase 10 landed dual-write slices (B/C/D/R1 first cuts) on `main`, but:

1. Local feature branches remain after merge (clutter; no unmerged commits).
2. Roadmap / PROJECT_SUMMARY / migration-guide still read as if dual-read is incomplete or pending.
3. Dual-mode test contract for **feature-off array authority when ECS mirrors diverge** is not explicitly asserted in one place — risk of silent semantic drift when later write-authority slices land.

This feature delivers **S1 cleanup + S2 dual-read acceptance** from `.mimocode/plans/next-phase-11-r1-authority.md`. Write-authority migration (S3) is out of scope here.

## [S2] Design

### Storage contract (unchanged)

| Mode | Read API | Write authority |
|------|----------|-----------------|
| `unity-world-primary` **off** (default) | Array / `GameObject` fields | Array; ECS mirrors may exist but are **not** read |
| `unity-world-primary` **on** | ECS component when linked + present, else array | Array write-through (`with_transform_mut`) or ECS-primary (`with_ecs_transform_mut`) |

### Dual-read inventory (already implemented in `world.rs`)

| Public read | Feature-on preferred source | Fallback |
|-------------|----------------------------|----------|
| `GetTransform` | ECS `Transform` | `GetTransformArray` |
| `GetName` | ECS `GameObjectName` | `gameobject_data` |
| `GetTag` | ECS `GameObjectTag` | `gameobject_data` |
| `IsActive` | ECS `GameObjectActive` | `gameobject_data` |
| `GetLayer` | ECS `GameObjectLayer` | `gameobject_data` |
| `GetParent` | ECS `GameObjectParent` | `GetParentArray` |
| `GetChildren` | ECS `GameObjectChildren` | `GetChildrenArray` |

`GetTransformArray` / `GetParentArray` / `GetChildrenArray` remain array-authoritative in **both** modes (scene I/O, hierarchy math).

### Contracts for this slice

1. **No production API redesign.** Existing dual-read preference stays; only docs + dual-mode tests.
2. **Divergence test** must pass under both feature modes with `World::unity_world_primary_feature()` branching:
   - off → public reads return **array** truth after ECS-only mutation
   - on → public reads return **ECS** truth after ECS-only mutation
3. **Workspace default features** stay `["audio"]` — do not enable `unity-world-primary` by default.
4. **Worktree override:** session policy blocked `git worktree add`; implement on local branch `phase11-r1-read` in the main checkout.
   **Branch-delete override:** session policy also blocked `git branch -d` on other refs; merged-branch cleanup deferred to the user/orchestrator (listed in Report).
5. **Editor smoke:** launch/run editor tests; full interactive Play verification is deferred if GPU/window is unavailable — report honestly.

### Verification gate

```bash
cargo test -p engine-core --lib
cargo test -p engine-core --lib --features unity-world-primary
cargo test -p engine-core --test unity_lifecycle_tests
cargo test -p engine-core --test unity_lifecycle_tests --features unity-world-primary
cargo test -p engine-editor --test editor_tests
```

## [S3] Out of Scope

- S3 write-authority slices (Transform/Hierarchy/Identity/MB authority move)
- Enabling `unity-world-primary` by default
- engine-scene Transform removal / animation keyframe cut (S4)
- VR/AR, Android NDK, WASM SceneRuntime full
- Mouse DPI root-cause fix (investigate only if time; not acceptance)
- Remote `git push`

## Tasks

- [ ] T1: Delete merged local branches — acceptance: blocked by session ref-store policy; list remains for user/orchestrator (covers: S1) — **blocked**
- [ ] T2: Sync README / PROJECT_SUMMARY / migration-guide / roadmap dual-read status — acceptance: docs state dual-read implemented; still-deferred items accurate (covers: S1)
- [ ] T3: Add dual-mode divergence acceptance test in `world.rs` — acceptance: test name visible; green with and without `unity-world-primary` (covers: S2)
- [ ] T4: Commit plan + feature doc + code/docs on `phase11-r1-read` — acceptance: clean status after commit (covers: S1; S2)
- [ ] T5: Run dual-mode verification gate — acceptance: listed commands PASS (covers: S2)
- [ ] T6: Editor smoke — acceptance: `cargo test -p engine-editor --test editor_tests` PASS; note if interactive run blocked (covers: S2)
