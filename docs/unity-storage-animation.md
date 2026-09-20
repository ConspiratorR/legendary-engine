# Unity 存储权威与动画路径

本文说明阶段 12–17 在 **`engine-core` 默认构建** 下的 Unity World 存储契约，以及动画 clip 如何写到 World 位姿。

## 存储权威（`unity-world-primary`）

`engine-core` 默认 features：`["audio", "unity-world-primary"]`。

| 存储族 | 默认构建权威 | 数组槽位角色 |
|--------|--------------|--------------|
| Identity（Name/Tag/Active/Layer） | ECS 镜像 | cache / 查找表 |
| Hierarchy（Parent/Children） | ECS 组件 | cache |
| Transform local 位姿 | ECS `Transform` | pose cache |
| Scene I/O | dual-read + **prepared** 刷新后序列化 | Load 后 seed → ECS 权威 |
| MonoBehaviour 元数据 | ECS `MonoBehaviourInstances` | 可重建 |
| MB 运行时实例 `Box<dyn>` | **数组 holder**（不迁 ECS） | 运行时唯一源 |

**Opt-out（数组权威）：**

```toml
engine-core = { path = "...", default-features = false, features = ["audio"] }
```

```bash
cargo test -p engine-core --lib --no-default-features --features audio
```

## Scene I/O

| API | 用途 |
|-----|------|
| `SaveSceneJsonPrepared` / `SceneSerializer::SavePrepared` / `SceneManager::SaveSceneJson` | **推荐**：先 `prepare_scene_io_cache` 再序列化 |
| `SaveSceneJson` / `SceneSerializer::Save` | 低层 `&World`，不刷新 cache；仅用于已准备好的 World |
| 编辑器 `save_scene_bundle` / `export_core_scene_json` | prepared（写 `.runtime.json`） |

## 动画写位姿

| 层 | 位置 | 角色 |
|----|------|------|
| Clip 格式 | `engine_scene::keyframe`（经 `engine_core::animation_apply` **再导出**） | 采样数学 |
| Apply | `engine_core::apply_clip_pose` | 写 World（`with_ecs_transform_mut`） |
| 运行时播放 | `engine_core::AnimationClipPlayer` | Update → time → apply |
| 编辑器动画面板 | 预览 → `apply_node_transform_to_world` | 同权威路径 |
| `engine_scene::transform` | 包内场景图 / legacy bridge | **非** gameplay 权威 |

示例：

```rust
use engine_core::{AnimationClip, apply_clip_pose, AnimationClipPlayer};
use engine_core::animation_apply::Vec3Keyframe;
```

## 物理与 World（阶段 18）

| 层 | 位置 | 角色 |
|----|------|------|
| Unity 组件 | `engine_core::components::Rigidbody` / colliders | 游戏侧数据 |
| 桥 | `engine_physics::unity_bridge`（`sync_physics_from_unity` / `to_unity` / `unity_physics_fixed_step`） | World ↔ physics ECS |
| 模拟 | `PhysicsWorld::step`（ECS `Transform` + physics `RigidBody`） | 积分/碰撞 |
| 写回 | `World::SetLocalPosition`（storage authority） | 位姿权威路径 |
| 插件 | `engine_physics::UnityPhysicsPlugin` | FixedUpdate：from → step → to |
| 示例 | `cargo run --example unity_physics_demo -p engine-core` | 重力下落写 World |

编辑器 Play 仍使用 `UnityPlayHost` 内的同构同步；运行时优先 `UnityPhysicsPlugin` + SceneRuntime。

## 可运行示例

```bash
# clip → prepared SceneData → SceneRuntime tick → World 位姿
cargo run --example unity_animation_demo -p engine-core

# Rigidbody 重力 → physics step → World 位姿
cargo run --example unity_physics_demo -p engine-core

# SceneData / Material 往返
cargo run --example runtime_scene_demo -p engine-core
```

## 相关规格

- `docs/compose/spec/phase12-array-authority.md` — R1-full 权威
- `docs/compose/spec/phase13-default-on-hardening.md` — 默认开 flag
- `docs/compose/spec/phase14-engine-scene-residual.md` — 播放器与再导出
- `docs/compose/spec/phase15-scene-io-hygiene.md` — prepared 调用收敛
- `docs/compose/spec/phase16-unity-animation-demo.md` — 动画端到端示例
- `docs/compose/spec/phase17-editor-anim-docs.md` — 编辑器 SceneData + 本文档
- `docs/compose/spec/phase18-unity-physics-bridge.md` — 运行时物理桥
- 契约表：`docs/migration-guide.md` §P2.4
