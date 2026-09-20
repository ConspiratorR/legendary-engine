# 分支索引 — `phase12-array-authority`（阶段 12–28）

合并/推送前核对用。规格均在 `docs/compose/spec/`。**git merge/push 由用户执行。**

| 阶段 | 主题 | 规格 | 门禁摘要 |
|------|------|------|----------|
| 12 | R1-full 数组权威迁 ECS（feature on） | `phase12-array-authority.md` | core lib 261 ON / opt-out；dual-read + write authority |
| 13 | 默认开 `unity-world-primary` + hop-cap + prepared save | `phase13-default-on-hardening.md` | default `["audio","unity-world-primary"]`；CI 双模态 |
| 14 | engine-scene 动画残余：再导出 + `AnimationClipPlayer` | `phase14-engine-scene-residual.md` | animation apply + player tests |
| 15 | Scene I/O prepared 卫生（tests/examples） | `phase15-scene-io-hygiene.md` | `SaveSceneJsonPrepared` 收敛 |
| 16 | `unity_animation_demo` 端到端 | `phase16-unity-animation-demo.md` | cargo run 位姿 0→8 |
| 17 | 编辑器 SceneData 动画恢复 + 文档 | `phase17-editor-anim-docs.md` | editor_tests 61→62 |
| 18 | Unity 物理桥 `unity_bridge` + `UnityPhysicsPlugin` | `phase18-unity-physics-bridge.md` | 自由落体 vy 累积 |
| 19 | 旋转写回 / Sleep / OnCollisionEnter | `phase19-physics-polish.md` | physics lib 75+ |
| 20 | relative_velocity / e2e / OnTriggerEnter + `is_trigger`→`is_sensor` | `phase20-collision-completeness.md` | frame enter events |
| 21 | WASM engine-core 基线（libloading 门控） | `phase21-wasm-engine-core.md` | wasm32 build PASS |
| 22 | WASM SceneRuntime API 面（JSON vs 文件） | `phase22-wasm-scene-runtime.md` | FromFile Err；hot-reload no-op |
| 23 | Exit 回调 + Capsule 桥 | `phase23-collision-exit-capsule.md` | enter+exit dispatch |
| 24 | Play MB 回调 + Capsule offset + 序列化 is_sensor | `phase24-play-callbacks-capsule-offset.md` | broadphase offset；rot*offset |
| 25 | Oriented Capsule `CapsuleAxis` X/Y/Z | `phase25-oriented-capsule.md` | direction→axis；segment 轴向 |
| 26 | Broadphase 旋转世界 AABB | `phase26-broadphase-world-aabb.md` | `rotated_aabb_half_extents`；center XOR half+offset |
| 27 | CCD 旋转障碍 AABB | `phase27-ccd-rotated-aabb.md` | ccd_sweep 与 broadphase 同构 |
| 28 | CCD 探针世界中心 + offset 写回 | `phase28-ccd-probe-awareness.md` | `center < 0`；origin writeback |

## 推荐门禁（合并前）

```powershell
cargo test -p engine-core --lib
cargo test -p engine-core --lib --no-default-features --features audio
cargo test -p engine-physics --lib
cargo test -p engine-physics --test physics_tests
cargo test -p engine-editor --test editor_tests
cargo build -p engine-core --examples
cargo build -p engine-core --target wasm32-unknown-unknown
cargo fmt -p engine-core -p engine-physics -p engine-editor --check
```

## 文档入口

- 存储权威与动画/物理：`docs/unity-storage-animation.md`
- 迁移契约：`docs/migration-guide.md` §P2.4
- WASM：`WASM_STATUS.md`
- README 阶段表：`README.md` 开发路线图

## 仍延后（非本分支承诺）

- dyn `MonoBehaviour` 进 ECS
- WASM 浏览器 SceneRuntime **全量** / Android NDK / VR·AR
- git merge → `main` / push / PR（用户自理）
