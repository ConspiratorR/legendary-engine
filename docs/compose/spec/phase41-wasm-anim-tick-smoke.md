---
feature: phase41-wasm-anim-tick-smoke
status: delivered
updated: 2026-07-13
branch: main
commits: a3830e8cc83e308b109f845e6b256efa0b74304f..impl
---

# Phase 41 — web-demo 动画 tick smoke

## Report

**What was built** — web-demo `scene_runtime_json_smoke` no longer ticks a static SceneData once. It loads SceneData with `AnimationClipPlayer` (`script_type` + props; position track 0→6 @1s, non-looping), runs 20× `Time::update(0.05)` + `SceneRuntime::tick`, and returns `start_x`/`end_x` for the `#scene-smoke` overlay (`anim tick ok; …`). Native regression `test_scenedata_anim_smoke_multitick` in engine-core uses the same JSON/multi-tick path and asserts X advances toward 6.0. WASM_STATUS / BRANCH_INDEX / README document phase 41; wasm-pack pkg still exports `scene_runtime_json_smoke`.

**Verification** —
- `cargo test -p engine-core --lib test_scenedata_anim_smoke_multitick` → PASS
- `cargo test -p engine-core --lib` → PASS 263
- `cargo test -p engine-core --lib --no-default-features --features audio` → PASS 242
- `cargo build --manifest-path examples/web-demo/Cargo.toml --target wasm32-unknown-unknown --lib` → PASS
- `pwsh scripts/run-compose-gates.ps1` → **11/11 PASS**
- `wasm-pack build --target web --release` → PASS；`pkg/web_demo.d.ts` exports `scene_runtime_json_smoke`
- Subagent review: T1 met; correctness sound; only critical was unfilled Report (this finalize)

**Journey log** —
1. Spec was already drafted untracked before implementation; kept in place, status flipped in-progress → delivered.
2. SceneData animation props contract: `properties.{script_type,enabled,props.{clip,time,speed,playing}}`; `playing` must be true in JSON (DeserializeProps defaults false).
3. Non-looping clip parks at `duration` after enough ticks (20×0.05s → X≈6.0); do not assert mid-clip without controlling tick count.
4. glam Vec3/Quat in SceneData/clip keys serialize as JSON arrays (`local_position`, keyframe `value`); `interpolation` is `"Linear"`.
5. Review confirmed code path; finalize Report is required delivery evidence for compose-next T2.

## [S1] Problem

web-demo smoke 只加载静态 SceneData 并 tick 一次；缺少 **wasm 上 AnimationClipPlayer 驱动 World 位姿变化** 的可见证据。

## [S2] Design

| 项 | 契约 |
|----|------|
| smoke JSON | 含 `script_type=AnimationClipPlayer` + clip props（position track 0→6 @1s） |
| smoke 逻辑 | `register_sample_scripts` → LoadSceneJson → 多次 `Time::update` + `tick` → 返回 **前后位姿** |
| overlay | 显示 start/end X，便于浏览器核对 |
| native 回归 | engine-core 测试同一多 tick 路径（X 前进） |
| 构建 | wasm-pack；gates 不回归 |

## [S3] Out of Scope

- 浏览器 UI 动画可视化渲染
- Android / git push（用户自理）

## Tasks

- [x] T1: smoke 多 tick 动画 + native 测试 — acceptance: 位姿前进断言（covers: S2）
- [x] T2: wasm-pack + 文档 + finalize — acceptance: Report（covers: S2; depends: T1）
