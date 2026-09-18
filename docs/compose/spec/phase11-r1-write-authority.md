---
feature: phase11-r1-write-authority
status: in-progress
updated: 2026-07-13
branch: phase11-r1-read
commits: 
---

# Phase 11 R1 — Write Authority Slices

## Report

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

No production redesign: `SetName`/`SetTag`/`SetActive`/`SetLayer`/`SetParent` already write array and ECS. S3 locks them with a dual-mode **post-write divergence** test:

- After a public write, ECS and array agree (coherent dual-write).
- Feature **on**: if only array is later corrupted, public read still returns the written (ECS) truth.
- Feature **off**: if only ECS is later corrupted, public read still returns the written (array) truth.

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

- [x] T1: Add World transform writers (`SetLocalPosition` / `SetLocalRotation` / `SetLocalScale` / `SetLocalPositionAndRotation`) routing through `with_ecs_transform_mut` — acceptance: methods compile; feature off still array-authoritative; feature on ECS-primary + array cache mirrored (covers: S2)
- [x] T2: Dual-mode transform write-authority test — `test_s3_transform_write_authority_after_set_local_position` (covers: S2; depends: T1)
- [x] T3: Dual-mode identity/hierarchy write-authority test — `test_s3_identity_write_authority_after_public_setters` (covers: S2)
- [x] T4: Keep MB restore + Instantiate seed regression green — existing B5/R1 tests still pass both feature modes (covers: S2)
- [x] T5: Sample scripts use World transform writers — `sample_scripts` Mover/Rotator call `with_ecs_transform_mut` (covers: S2; depends: T1)
- [x] T6: Update `docs/migration-guide.md` write-authority contract — feature on = ECS read authority + array cache; feature off = array authority (covers: S2)
- [x] T7: Dual-mode verification gate + examples build — listed commands PASS (covers: S2)
