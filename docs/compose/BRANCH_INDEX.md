# 分支索引 — Unity 对齐长分支（阶段 12–45）

合并/推送前核对用。规格均在 `docs/compose/spec/`。**已出现在本地 `main` 时以 main 为准；远程 merge/push 由用户执行。**

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

| 29 | 分支索引 12–28 | `phase29-branch-index.md` | BRANCH_INDEX + PROJECT_SUMMARY |
| 30 | Android 基线探测 + `game-activity` feature | `phase30-android-baseline.md` | 本机缺 NDK；文档/feature 就绪 |
| 31 | WASM physics + CI 编译面 | `phase31-wasm-physics-ci.md` | engine-physics wasm32 ✅；CI 增 core/physics |
| 32 | 合并前门禁脚本 | `phase32-branch-gates-script.md` | `scripts/run-compose-gates.ps1/.sh` |
| 33 | Inspector 物理字段 | `phase33-inspector-physics-fields.md` | 碰撞体尺寸/中心/轴向/触发器/Sleep |
| 34 | 移除菜单 + 多碰撞体说明 | `phase34-editor-menu-docs.md` | 粒度移除；Sphere→Box→Capsule |
| 35 | unity_sim_demo 综合示例 | `phase35-unity-sim-demo.md` | 动画+物理+SceneData 一跑 |
| 36 | web-demo SceneRuntime WASM | `phase36-wasm-scene-runtime-demo.md` | JSON smoke 编译进 web-demo |
| 37 | SceneData smoke 回归 + wasm-pack | `phase37-scenedata-smoke-regression.md` | native 262；pkg 含 smoke 导出 |
| 38 | 全量门禁落地记录 | `phase38-landing-gates.md` | compose-gates **11/11 PASS** |
| 39 | web-demo smoke 可视化 | `phase39-web-demo-smoke-ui.md` | `#scene-smoke`；smoke 先于 wgpu |
| 40 | 落地验证 wasm-pack + 门禁 | `phase40-landing-verify.md` | pkg 刷新；gates **11/11 PASS**（main） |
| 41 | web-demo 动画 tick smoke | `phase41-wasm-anim-tick-smoke.md` | AnimationClipPlayer multi-tick；native + wasm overlay |
| 42 | 多碰撞体进物理桥 | `phase42-multi-collider-bridge.md` | primary 优先级 + secondary kinematic 实体 |
| 43 | secondary 事件映射父 MB | `phase43-secondary-callback-map.md` | go_for_physics_id → SecondaryCollider.parent |
| 44 | 同 GO 碰撞对事件抑制 | `phase44-same-go-event-suppress.md` | dispatch a==b skip（Unity 对齐） |
| 45 | SceneData 多碰撞体往返→桥 | `phase45-scenedata-multicollider-roundtrip.md` | prepared save/load 后 primary+secondary |

## 推荐门禁（合并前）

一键脚本（与下方命令一致）：

```powershell
pwsh scripts/run-compose-gates.ps1
```

```bash
bash scripts/run-compose-gates.sh
```

手动命令：

```powershell
cargo test -p engine-core --lib
cargo test -p engine-core --lib --no-default-features --features audio
cargo test -p engine-physics --lib
cargo test -p engine-physics --test physics_tests
cargo test -p engine-editor --test editor_tests
cargo build -p engine-core --examples
cargo build -p engine-core --target wasm32-unknown-unknown --no-default-features --features unity-world-primary
cargo build -p engine-physics --target wasm32-unknown-unknown
cargo build -p engine-editor --target wasm32-unknown-unknown --no-default-features --lib
cargo build --manifest-path examples/web-demo/Cargo.toml --target wasm32-unknown-unknown --lib
cargo fmt -p engine-core -p engine-physics -p engine-editor --check
```

## 文档入口

- 存储权威与动画/物理：`docs/unity-storage-animation.md`
- 迁移契约：`docs/migration-guide.md` §P2.4
- WASM：`WASM_STATUS.md`
- README 阶段表：`README.md` 开发路线图

## 仍延后（非本分支承诺）

- dyn `MonoBehaviour` 进 ECS
- WASM 浏览器 SceneRuntime **全量 UI**（phase 41 仅动画 tick 证据）/ Android NDK / VR·AR
- compound 冲量（phase 42 边界；phase 43 已补 secondary→父 MB；phase 44 同 GO 抑制已对齐 Unity）
- git merge → `main` / push / PR（用户自理）
