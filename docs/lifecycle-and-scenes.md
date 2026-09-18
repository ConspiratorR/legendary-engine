# Unity 对齐：生命周期与场景系统

本文档描述 RustEngine 如何对齐 Unity 的核心运行时契约（参考本地 `UnityDocumentation/`），
以及游戏代码应如何使用这些 API。

## 架构分层

```
App (ECS world + schedule + Time + EventBus)
 └── SceneRuntime 资源
      ├── World          — Unity 风格 GameObject / Transform / MonoBehaviour
      └── SceneManager   — 场景加载 / 卸载 / DontDestroyOnLoad
```

- **ECS World**（`engine_ecs`）：系统调度、渲染/物理等数据密集型组件。
- **Unity World**（`engine_core::world::World`）：脚本与场景图，挂在 `SceneRuntime` 资源上。
- 两套 World 并存是过渡方案；编辑器与脚本 API 走 Unity World，高性能系统走 ECS。

## 帧生命周期（对齐 Unity PlayerLoop）

`App::run_with_lifecycle(delta)` 每帧执行：

```
1. Time::update(delta)              // 填充 FixedUpdate 累加器
2. InputManager::update_frame()
3. pre-update hooks
4. FixedUpdate 0+ times
   - begin_fixed_update
   - **fixed ECS schedule**（物理等，`add_fixed_ecs_system`）
   - PlayerLoop FixedUpdate 相位
   - MonoBehaviour.FixedUpdate
   - end_fixed_update
5. Update
   - PlayerLoop Update 相位
   - ECS schedule
   - MonoBehaviour.Start (首次) + Update
6. LateUpdate
   - PlayerLoop LateUpdate 相位
   - MonoBehaviour.LateUpdate
7. End of frame
   - flush OnEnable/OnDisable（来自 SetActive）
   - tick_coroutines / tick_invokes
   - update_pending_destroy
   - sync_transforms
   - flush_destroy（OnDisable → OnDestroy → 释放；并取消该对象 Invoke/Coroutine）
8. post-update hooks
```

物理请用 `AppBuilder::add_fixed_ecs_system` 注册，每步 FixedUpdate 以 `Time.fixedDeltaTime` 推进。

对应 Unity 文档：
- [Execution Order](https://docs.unity3d.com/Manual/ExecutionOrder.html)
- [Time / fixedDeltaTime](https://docs.unity3d.com/ScriptReference/Time.html)

### 固定步长

```rust
use engine_core::time::Time;

let mut time = Time::default(); // fixedDeltaTime = 0.02 (50 Hz)
time.update(0.016);             // 一帧
assert_eq!(time.pending_fixed_steps(), 0); // 累加不足一步

time.update(0.01);
assert_eq!(time.pending_fixed_steps(), 1); // 可跑一次 FixedUpdate
```

- `timeScale = 0` 时 FixedUpdate 步数为 0（暂停）。
- `maximumDeltaTime` 钳制单帧 delta；`max_fixed_steps`（默认 8）防止螺旋死亡。

## 场景管理

```rust
use engine_core::{SceneRuntime, LoadSceneMode};
use engine_core::plugins::{CorePlugins, SceneRuntimePlugin};

// CorePlugins 已包含 SceneRuntimePlugin
app.add_plugin(CorePlugins);

// 运行时
let rt = app.scene_runtime_mut().unwrap();
let handle = rt.load_scene_json("Level1", json, LoadSceneMode::Additive)?;
rt.unload_scene(handle)?;
```

| Unity API | RustEngine |
|-----------|------------|
| `SceneManager.LoadScene` | `SceneManager::LoadSceneJson` / `LoadSceneFromFile` |
| `LoadSceneMode.Single/Additive` | `LoadSceneMode::{Single, Additive}` |
| `SceneManager.UnloadScene` | `UnloadSceneWithWorld`（销毁场景根） |
| `Object.DontDestroyOnLoad` | `World::DontDestroyOnLoad`，卸载时跳过 |

场景 JSON 使用 glam 数组格式：

```json
{
  "name": "Level",
  "version": 1,
  "game_objects": [{
    "name": "Player",
    "tag": "Player",
    "layer": 0,
    "active": true,
    "transform": {
      "local_position": [1.0, 2.0, 3.0],
      "local_rotation": [0.0, 0.0, 0.0, 1.0],
      "local_scale": [1.0, 1.0, 1.0]
    },
    "components": [],
    "children": []
  }]
}
```

## GameObject 与生命周期回调

```rust
use engine_core::world::World;
use engine_core::{MonoBehaviour, Behaviour, Component, Context};
use std::any::Any;

struct Player { speed: f32 }

impl Component for Player {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

impl Behaviour for Player {
    fn Enabled(&self) -> bool { true }
    fn SetEnabled(&mut self, _e: bool) {}
    fn IsActiveAndEnabled(&self) -> bool { true }
    fn set_gameobject(&mut self, _h: GameObjectHandle) {}
    fn gameobject_handle(&self) -> Option<GameObjectHandle> { None }
}

impl MonoBehaviour for Player {
    fn Start(&mut self, _ctx: &mut Context) {
        // Unity Start
    }
    fn Update(&mut self, ctx: &mut Context) {
        let dt = ctx.DeltaTime();
        // 移动逻辑
    }
    fn FixedUpdate(&mut self, _ctx: &mut Context) {
        // 物理相关
    }
}

let mut world = World::new();
let player = world.CreateGameObject("Player");
world.AddMonoBehaviour(player, Player { speed: 5.0 });
```

### SetActive / OnEnable / OnDisable

```rust
world.SetActive(handle, false);
// OnEnable/OnDisable 入队，在下一次 lifecycle tick（帧末）统一 flush
assert_eq!(world.pending_enable_disable_count(), 1);

// 需要当场回调时（编辑器/测试）：
world.SetActiveImmediate(handle, false, time, frame, &mut events);
assert_eq!(world.pending_enable_disable_count(), 0);
```

- 父物体 `SetActive(false)` 会使子物体 `activeInHierarchy == false` 并级联 OnDisable。
- 子物体 `activeSelf` 保持不变，符合 Unity 语义。
- `AddMonoBehaviour` 的脚本可通过 `GetComponent::<T>()` / `HasComponent::<T>()` 访问（与 Unity「脚本即组件」一致）。

### Destroy

```rust
world.Destroy(handle);              // 帧末销毁
world.DestroyDelayed(handle, 2.0);  // 2 秒后销毁
world.DestroyImmediate(handle);     // 立即销毁（编辑器/场景卸载）
```

帧末会调用 OnDisable → OnDestroy，再释放句柄。

### RequireComponent

```rust
impl Component for Mesh {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
    fn required_on_add(&self) -> Vec<Box<dyn Fn() -> Box<dyn Component>>> {
        vec![Box::new(|| Box::new(MeshRenderer))]
    }
}

world.AddComponent(handle, Mesh); // 自动补上 MeshRenderer
```

### Invoke

```rust
world.Invoke(handle, "Explode", 1.5);
world.InvokeRepeating(handle, "Pulse", 0.0, 0.5);
world.CancelInvoke(handle);
world.CancelInvokeMethod(handle, "Pulse");
world.IsInvoking(handle);
```

到时通过 `SendMessage` 分发。

## Coroutine

Rust 无稳定版生成器，协程建模为显式步进列表：

```rust
use engine_core::{CoroutineStep, World};

let mut world = World::new();
let obj = world.CreateGameObject("Flash");

let id = world.StartCoroutine(obj, "Blink", vec![
    CoroutineStep::Wait(0.25),
    CoroutineStep::SetActive(false),
    CoroutineStep::Wait(0.25),
    CoroutineStep::SetActive(true),
]);

// 每帧由 lifecycle tick 自动推进
world.StopCoroutine(id);
world.StopAllCoroutines(obj);
```

| Step | 对应 Unity |
|------|------------|
| `Wait(n)` | `yield return new WaitForSeconds(n)`（受 `timeScale` 影响） |
| `WaitRealtime(n)` | `yield return new WaitForSecondsRealtime(n)`（忽略 `timeScale`） |
| `WaitEndOfFrame` | `yield return null` / `WaitForEndOfFrame` |
| `WaitFixedUpdate` | `yield return new WaitForFixedUpdate()`（本帧有 FixedUpdate 则恢复） |
| `WaitUntil(pred)` | `yield return new WaitUntil(pred)` |
| `WaitWhile(pred)` | `yield return new WaitWhile(pred)` |
| `SetActive(b)` | 脚本内改 active |
| `Call(name)` | `SendMessage` |
| `Action(f)` | 自定义闭包 |

脚本内可通过 `Context` 启动/停止协程（对应 `MonoBehaviour.StartCoroutine` / `StopCoroutine`）：

```rust
impl MonoBehaviour for Player {
    fn Start(&mut self, ctx: &mut Context) {
        let me = self.gameobject_handle().unwrap();
        ctx.StartCoroutine(me, "Regen", vec![
            CoroutineStep::Wait(1.0),
            CoroutineStep::Call("Heal".into()),
        ]);
    }
}
```

停止：

```rust
world.StopCoroutine(id);                 // Coroutine 句柄
world.StopCoroutineByName(obj, "Blink"); // Unity StopCoroutine(string)
world.StopAllCoroutines(obj);            // MonoBehaviour.StopAllCoroutines
```

场景卸载时会 `Destroy` 非 DDOL 根节点，其协程随之停止；`DontDestroyOnLoad` 对象的协程继续运行。
可运行示例：`cargo run -p engine-core --example coroutine_demo`

### SendMessage

```rust
impl MonoBehaviour for Enemy {
    fn on_message(&mut self, method: &str, value: Option<&dyn Any>, _ctx: &mut Context) {
        match method {
            "Hit" => { /* value.downcast_ref::<i32>() */ }
            _ => {}
        }
    }
}

world.SendMessage(obj, "Hit");
world.SendMessageWithValue(obj, "Hit", &10i32);
// Invoke / Coroutine::Call 也走 on_message
```

## Prefab Variant

```rust
use engine_core::prefab::{Prefab, PrefabNodeOverride};

let base = Prefab::Create("EnemyBase", &go, &world);
let mut variant = Prefab::CreateVariant("EnemyBoss", &base);
variant.override_node("", PrefabNodeOverride {
    tag: Some("Boss".into()),
    layer: Some(3),
    ..Default::default()
});
variant.override_node("Weapon", PrefabNodeOverride {
    name: Some("Sword".into()),
    add_components: vec!["Blade".into()],
    ..Default::default()
});

let instance = variant.Instantiate(&mut world);
```

- 路径从根节点子级起算：根为 `""`，子节点为其名字，更深为 `Parent/Child`。
- `Instantiate` / `merged_root()` 自动合并 override；`apply_overrides()` 可物化到存储树。

## ScriptableObject 资产文件

Unity 的 `.asset` + `.meta`（GUID）模型：

```text
Assets/
  Goblin.asset          # 序列化载荷
  Goblin.asset.meta     # 稳定 GUID
```

```rust
use engine_core::asset_database::AssetDatabase;
use engine_core::scriptable_asset::{save_scriptable_object, load_scriptable_object};

// 底层 API
let meta = save_scriptable_object(path, "Goblin", &data)?;
let data: EnemyData = load_scriptable_object(path)?;

// AssetDatabase 集成
let mut db = AssetDatabase::new();
db.set_assets_root("Assets");
db.save_asset_to_disk(None, "Goblin", &data)?;
db.scan_assets_root()?;
let handle = db.load_asset_by_guid::<EnemyData>(&meta.guid)?;
```

- 重复保存会 **保留原 GUID**（文件移动/重写不丢引用）。
- 目录扫描自动为缺失的 `.meta` 补发 GUID。

### AssetRef（GUID 引用）

组件/场景字段里存对另一资产的引用，序列化为 GUID 字符串：

```rust
use engine_core::AssetRef;

// 从数据库取引用
let r: AssetRef = db.asset_ref("Goblin");

// 解析回句柄
let handle = db.resolve_ref::<EnemyData>(&r)?;

// 序列化：\"9f8a7b...\"
```

空 GUID 表示 null 引用（Unity `null`）。

### 热重载

`save` / `load` 会自动登记 watch。每帧或定时调用：

```rust
let events = db.poll_hot_reload();
for e in events {
    // e.guid, e.name, e.path — 资产已从磁盘重新载入 entries
}
```

- 基于 mtime 比较，无后台线程；文件被外部修改后下次 poll 生效。
- 内存中旧的 `Arc` 句柄仍指向旧数据；新的 `get_asset` / `resolve_ref` 得到新数据。

### 与 engine-asset 的关系（双轨共存）

| | `AssetDatabase`（本模块 / `.asset`） | `engine_asset::Registry` |
|--|--------------------------------------|---------------------------|
| 定位 | 游戏数据 ScriptableObject（配置、数值） | 运行时资源（贴图、网格句柄） |
| 标识 | 稳定 **GUID**（`.meta`） | 内存 `Handle<T>` / 资产 key |
| 序列化 | `.asset` JSON + `.meta` | 通常不进场景文件 |
| 场景引用 | `AssetRef { guid }`（组件属性） | 不写入 SceneData |

两套系统可并行：场景里用 `AssetRef` 指向共享数值资产；渲染用 `engine_asset` 贴图。桥接（Handle ↔ GUID）标为后续 Phase，不在本迭代强并。

CLI（P3.5）：

```bash
cargo run -p engine-core --bin so_asset -- pack Assets   # 递归补 .meta
cargo run -p engine-core --bin so_asset -- list Assets   # path / name / guid
```

## 身份桥（GameObject ↔ Entity）

编辑器/脚本用 `GameObjectHandle`，渲染/物理用 ECS `Entity`。身份桥在**不合并存储**的前提下建立双向映射：

```rust
// 创建并立即登记
let (go, entity) = app.spawn_linked("Player")?;

// 普通 spawn 也会在下一帧 sync 时被自动 adopt
let go = app.unity_world().unwrap().CreateGameObject("Later");
app.link_unity_scene(); // 或等 run_with_lifecycle

// 查询
app.entity_for_gameobject(go);
app.gameobject_for_entity(entity);
app.unity_world_ref();
app.require_unity_world()?; // SceneRuntime 缺失时返回 Err

// 每帧由 run_with_lifecycle 自动 sync（prune + 全量 adopt + 代理同步）
app.sync_identity_bridge();
```

同步结果（ECS 组件）：

| 组件 | 来源 |
|------|------|
| `TransformProxy` | Unity Transform 世界位姿 |
| `RenderProxy` | Material / SpriteRenderer（color、sprite 路径、flip） |

`App::render_phase` 会把 `TransformProxy`+`RenderProxy` 合并成 `engine_render::sprite::Sprite`（1×1 白纹理兜底），与 ECS `Sprite` 组件一并提交。

模块：`engine_core::identity_bridge`（`IdentityBridge`, `TransformProxy`, `RenderProxy`, `collect_proxy_sprites`）。

编辑器保存：`EditorState::save_scene_bundle` 写出编辑器 Scene + `<path>.runtime.json`（`SceneData`）；`open_scene_file` 优先识别 `game_objects` 字段。

MonoBehaviour 可进 SceneData：实现 `SerializeProps` / `DeserializeProps`，并 `register_mono_behaviour::<T>()`；保存写 `script_type` + props，加载经全局注册表还原。

编辑器 Play：进入运行时 `build_unity_play_host` 从编辑器 World 克隆层级到 `SceneRuntime`，每帧 `tick_unity_play_host` 走 Unity 生命周期；停止时丢弃。

## 与 Unity 的对应关系

| Unity | RustEngine |
|-------|------------|
| `GameObject` | `engine_core::gameobject::GameObject` + `GameObjectHandle` |
| `Transform` | `engine_core::transform::Transform`（每个 GO 内置） |
| `MonoBehaviour` | `engine_core::monobehaviour::MonoBehaviour` trait |
| `Time.deltaTime` | `Time::deltaTime()` / `Context::DeltaTime()` |
| `Time.fixedDeltaTime` | `Time::fixedDeltaTime()` |
| `Time.timeScale` | `Time::timeScale()` |
| `Object.Destroy` | `World::Destroy` / `DestroyDelayed` |
| `Object.DontDestroyOnLoad` | `World::DontDestroyOnLoad` |
| `RequireComponent` | `Component::required_on_add` |
| `Invoke` | `World::Invoke` |
| `StartCoroutine` | `World::StartCoroutine` + `CoroutineStep` |
| `WaitForSeconds` | `CoroutineStep::Wait` |
| Prefab Variant | `Prefab::CreateVariant` + `override_node` |
| `ScriptableObject` 资产 | `.asset` + `.meta` GUID（`scriptable_asset`） |
| `AssetDatabase` | `engine_core::asset_database::AssetDatabase` |
| 资产引用 | `AssetRef`（GUID） |
| 资产热重载 | `AssetDatabase::poll_hot_reload` |
| `SceneManager` | `engine_core::scene_management::SceneManager` |
| PlayerLoop | `App::run_with_lifecycle` + `PlayerLoop` 相位 |

## 后续路线（未完成）

详细分阶段执行计划见 [unity-alignment-roadmap.md](unity-alignment-roadmap.md)。

1. **P1 / P3 / P4 / 编辑器主路径** — ✅ 已在 main  
2. **P2.4 dual-write 切片** — ✅ 写通 Transform/Hierarchy/MBInstances、Destroy despawn、CI 双开测；**dual-read 优先 ECS**（Name/Tag/Active/Layer/Parent/Children/Transform）✅；`gameobject_data`/`transforms`/`monobehaviours` **数组写权威仍保留**，完整写权威迁移 → 阶段 11 R1 续  
3. **P2.5 engine-scene Transform** — 盘点 + 模块 doc 完成；`light_collect_system` 已优先 `TransformProxy`；animation_editor keyframe 仍用 engine-scene — 完整替换延后  
4. **D2 样例脚本** — ✅ `sample_scripts`（Mover/Rotator/Lifetime）随 `CorePlugins` 注册进 SceneData  

试验开 flag 与双读契约见 [migration-guide.md](migration-guide.md)。

## 相关源码

| 模块 | 路径 |
|------|------|
| Time | `crates/engine-core/src/time.rs` |
| App 主循环 | `crates/engine-core/src/app.rs` |
| Unity World | `crates/engine-core/src/world.rs` |
| SceneRuntime | `crates/engine-core/src/scene_runtime.rs` |
| SceneManager | `crates/engine-core/src/scene_management.rs` |
| MonoBehaviour | `crates/engine-core/src/monobehaviour.rs` |
| Coroutine | `crates/engine-core/src/coroutine.rs` |
| Prefab / Variant | `crates/engine-core/src/prefab.rs` |
| ScriptableObject 资产 | `crates/engine-core/src/scriptable_asset.rs` |
| AssetDatabase | `crates/engine-core/src/asset_database.rs` |
| PlayerLoop | `crates/engine-core/src/player_loop.rs` |
| 编辑器 Scene 桥 | `EditorState::to_core_scene_data` / `import_core_scene_json`（`engine-editor/src/state.rs`） |

编辑器 3D 视口：`build_scene` 优先读 Unity World 的 Transform；Play 时 `tick_unity_play_host` 全层级镜像回编辑器 World，并 `sync_node_transforms_from_world` 更新快照。

```rust
// 编辑器导出 → 运行时加载
let json = editor.export_core_scene_json("Level1")?;
runtime.load_scene_json("Level1", &json, LoadSceneMode::Single)?;

// 运行时场景 JSON → 编辑器打开
editor.load_core_scene_file(Path::new("Assets/Level1.scene.json"))?;
```
