---
feature: phase21-wasm-engine-core
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 0702dff..HEAD
---

# Phase 21 — WASM 构建基线：engine-core wasm32

## Report

**What was built** — `engine-core` now compiles for `wasm32-unknown-unknown`. `libloading` moved to `cfg(not(target_arch = "wasm32"))` dependencies. Dynamic plugin APIs (`DynamicPlugin::load`, `PluginLoader::load_all`, `AppBuilder::load_dynamic_plugins`) return `PluginLoadError::UnsupportedPlatform` on WASM; `PluginManifest`/`PluginRegistry` stay available. `WASM_STATUS.md` records engine-core ✅ + phase 21 gates; README 阶段 21 notes SceneRuntime browser full still deferred.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo build -p engine-core --target wasm32-unknown-unknown` | **PASS** |
| `cargo build -p engine-core --target wasm32-unknown-unknown --no-default-features` | PASS |
| `cargo build -p engine-core --target wasm32-unknown-unknown --no-default-features --features unity-world-primary` | PASS |
| `cargo test -p engine-core --lib` | PASS 261 |
| `cargo test -p engine-physics --lib` | PASS 78 |
| Native `cargo build -p engine-core` | PASS |

**Journey log** —
1. First wasm failure was solely `libloading` types in plugin_loader — gate that dep.
2. Native dynamic plugins unchanged when not wasm32.
3. SceneRuntime browser runtime is a **later** R3 slice, not this baseline.
4. git merge/push not handled per user preference.

## [S1] Problem
engine-core did not compile on wasm32 because of libloading.

## [S2] Design
Target-gated libloading + UnsupportedPlatform for dynamic plugin APIs.

## [S3] Out of Scope
WASM SceneRuntime full, Android NDK, git merge/push

## Tasks

- [x] T1: libloading / 插件路径 wasm 门控 (covers: S2)
- [x] T2: WASM_STATUS + README (covers: S2; depends: T1)
- [x] T3: native 门禁 + finalize (covers: S2; depends: T1,T2)
