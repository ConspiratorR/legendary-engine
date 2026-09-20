---
feature: phase22-wasm-scene-runtime
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 6da4162..HEAD
---

# Phase 22 — WASM SceneRuntime：JSON 路径与文件 API 门控

## Report

**What was built** — WASM-safe SceneRuntime surface documented and gated: `LoadSceneJson` + tick remain the recommended path (compiles on wasm32). `SceneManager::LoadSceneFromFile` returns a clear error on WASM. `AssetDatabase::poll_hot_reload` is a no-op on WASM. `save_scriptable_object` / `load_scriptable_object` return errors on WASM (native file I/O unchanged). WASM_STATUS + README describe JSON vs file APIs.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo build -p engine-core --target wasm32-unknown-unknown` | PASS |
| `cargo test -p engine-core --lib` | PASS 261 |

**Journey log** —
1. wasm32 compiles `std::fs` but has no reliable host FS — gate file APIs explicitly.
2. SceneRuntime gameplay on WASM = embed/load **JSON strings**, not disk paths.
3. Browser WebGL SceneRuntime full remains later R3 work.
4. git merge/push not handled per user preference.

## [S1] Problem
WASM compiles SceneRuntime but file-based scene/asset APIs are unusable without explicit gates/docs.

## [S2] Design
JSON-safe APIs stay; file/asset paths fail or no-op on WASM.

## [S3] Out of Scope
Browser WebGL SceneRuntime full, Android NDK, git merge/push

## Tasks

- [x] T1: 文件场景/资产 wasm 门控 (covers: S2)
- [x] T2: WASM_STATUS SceneRuntime 面 (covers: S2; depends: T1)
- [x] T3: native 测试 + wasm build + finalize (covers: S2; depends: T1,T2)
