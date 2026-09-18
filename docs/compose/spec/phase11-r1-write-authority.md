---
feature: phase11-r1-write-authority
status: delivered
updated: 2026-07-13
branch: phase11-r1-read
commits: c756def877ae3e805d3e77d167dbb1911fadc13c..HEAD
---

# Phase 11 R1 — Write Authority Slices

## Report

**What was built** — Phase 11 S3 write-authority on `phase11-r1-read`. Public World transform writers (`SetLocalPosition` / `SetLocalRotation` / `SetLocalScale` / `SetLocalPositionAndRotation`) route through `with_ecs_transform_mut`: feature off keeps array authority; feature on mutates ECS `Transform` and mirrors local pose to the array cache. Identity setters already dual-write; dual-mode tests lock post-write authority by corrupting the **non-authoritative** store and asserting public reads still return the written truth. `sample_scripts` Mover/Rotator use `with_ecs_transform_mut` for relative motion. `migration-guide` documents the write-authority table and public writers. Workspace default features remain `["audio"]`.

**Verification** —
- `cargo test -p engine-core --lib` — PASS (222)
- `cargo test -p engine-core --lib --features unity-world-primary` — PASS (242)
- `cargo test -p engine-core --lib test_s3_` (both modes) — PASS (2 + 2)
- `cargo test -p engine-core --test unity_lifecycle_tests` — PASS (21)
- `cargo test -p engine-core --test unity_lifecycle_tests --features unity-world-primary` — PASS (23)
- `cargo test -p engine-editor --test editor_tests` — PASS (58)
- `cargo build -p engine-core --examples` — PASS

**Journey log** —
1. S3 production surface for identity was already dual-write; real new API is World `SetLocal*` wrappers over `with_ecs_transform_mut`.
2. Dual-mode tests use `World::unity_world_primary_feature()` at runtime so one body covers both CI feature jobs.
3. Reviewer noted vacuous-pass risk if corruption were a no-op — tests now assert the corrupted store (`GetTransformArray` / array GameObject fields) before asserting public authority.
4. `SetParent` still syncs ECS hierarchy only when the feature is compiled on — intentional; feature-off dual-read ignores ECS parent anyway.
5. `sample_scripts` correctly use `with_ecs_transform_mut` for RMW motion rather than absolute `SetLocal*`.

## [S1] Problem

Under `unity-world-primary`, public **reads** already prefer ECS mirrors (S2). Write paths still mostly mutate **array storage** via `with_transform_mut` (array-primary + ECS write-through). That keeps dual-read coherent when write-through runs, but does not express the intended end-state contract:

- **feature off (default):** array / `GameObject` fields are the **authority**; ECS mirrors may lag and must not affect public reads.
- **feature on:** ECS components are the **read authority**; array pose/identity fields are **cache**. Transform mutation should go through the ECS-primary path so array cannot silently become stale relative to dual-read.

Identity APIs (`SetName` / `SetTag` / `SetActive` / `SetLayer` / `SetParent`) already dual-write; they need an explicit dual-mode **write-authority** acceptance test (including array/ECS divergence after a public write). Transform convenience writers on `World` that route through `with_ecs_transform_mut` are missing.

## [S2] Design

### Write-authority contract

| Mode | Transform mutation | Identity / hierarchy mutation | Public reads after write |
|------|--------------------|-------------------------------|---------------------------|
| off | `with_transform_mut` / `GetTransformMut` (array) | array + best-effort ECS mirrors | **array** (S2) |
| on | `with_ecs_transform_mut` (ECS primary) + array pose cache | array + ECS dual-write; dual-read prefers ECS | **ECS** (S2) |

### New World transform writers (R1e)

All route through [`World::with_ecs_transform_mut`], which already:

- feature **off** → falls back to `with_transform_mut` (array authority)
- feature **on** → mutates ECS `Transform`, then `sync_transform_from_ecs` mirrors local pose onto array cache

```rust
world.SetLocalPosition(handle, pos);
world.SetLocalRotation(handle, rot);
world.SetLocalScale(handle, scale);
world.SetLocalPositionAndRotation(handle, pos, rot);
```

`GetTransformMut` remains array-only (documented). Internal `sync_transform_recursive` may keep using array mut for hierarchy world-pose math; `sync_transforms` already `sync_all_transforms_to_ecs` when the feature is on.

### Identity / hierarchy write authority

No production redesign: `SetName`/`SetTag`/`SetActive`/`SetLayer`/`SetParent` already write array and ECS (SetParent hierarchy mirrors are feature-gated). S3 locks them with a dual-mode **post-write divergence** test that also asserts the corrupted store.

### MonoBehaviour metadata

- Array `monobehaviours` holders remain **instance authority**.
- ECS `MonoBehaviourInstances` / `MonoBehaviourTypes` are recoverable mirrors (B5).
- Add/Restore keep mirrors in sync; `restore_monobehaviours_from_ecs` rebuilds holders from mirrors + registry (existing test retained).

### Instantiate / Load seed

Unchanged from R1: `InstantiateAtPosition` ends with `seed_ecs_from_array`; scene load seeds mirrors. Covered by existing tests; S3 regression only.

### Out of scope for this document

- S4 animation keyframe / engine-scene cut
- S5 editor interactive Play / mouse DPI
- Default-on `unity-world-primary`
- Deleting array storage fields
- VR/AR / Android / WASM SceneRuntime

## [S3] Out of Scope

See design out-of-scope. Additionally: no workspace default-feature flip; no render Pass changes; no `git push`; no branch-ref deletion (session policy).

## Tasks

- [x] T1: Add World transform writers routing through `with_ecs_transform_mut` (covers: S2)
- [x] T2: Dual-mode transform write-authority test — `test_s3_transform_write_authority_after_set_local_position` (covers: S2; depends: T1)
- [x] T3: Dual-mode identity/hierarchy write-authority test — `test_s3_identity_write_authority_after_public_setters` (covers: S2)
- [x] T4: Keep MB restore + Instantiate seed regression green both feature modes (covers: S2)
- [x] T5: Sample scripts use `with_ecs_transform_mut` (covers: S2; depends: T1)
- [x] T6: Update `docs/migration-guide.md` write-authority contract (covers: S2)
- [x] T7: Dual-mode verification gate + examples build (covers: S2)
