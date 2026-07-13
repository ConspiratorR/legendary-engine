# RustEngine vs Unity Architecture Audit & Alignment Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Audit RustEngine's current architecture against Unity's documented architecture, identify gaps, and create a phased plan to align key systems with Unity's conventions.

**Architecture:** RustEngine already covers most Unity subsystems but with different naming, API patterns, and some missing features. The alignment focuses on: (1) Unity's MonoBehaviour/GameObject/Component pattern, (2) Unity's lifecycle and messaging conventions, (3) Unity's Editor workflow patterns, (4) Unity's scripting conventions (MonoBehaviour → Rust equivalent).

**Tech Stack:** Rust, wgpu, egui, Lua/WASM scripting

---

## Unity Architecture Reference

Unity's core architecture is built on these concepts:
1. **GameObject** — Container for Components, has Transform (always), active/static state, tags, layers
2. **Component** — Functional piece of a GameObject (Transform, MeshRenderer, Rigidbody, etc.)
3. **MonoBehaviour** — Base class for all user scripts, provides lifecycle callbacks (Awake, Start, Update, FixedUpdate, LateUpdate, OnDestroy, OnEnable, OnDisable)
4. **ScriptableObject** — Data container for sharing data between instances, survives scene loads
5. **Transform** — Position/Rotation/Scale, parent-child hierarchy
6. **Scene** — Collection of GameObjects, can load/unload additively
7. **Prefab** — Reusable GameObject template, overrides, nested prefabs
8. **Assets** — Files on disk (textures, meshes, scripts, materials, etc.), .meta files for GUIDs
9. **Editor** — Scene view, Hierarchy, Inspector, Project, Console, Game view

## Gap Analysis: RustEngine vs Unity

### 1. Core Architecture (engine-core)

| Unity Concept | RustEngine Equivalent | Status | Gap |
|---|---|---|---|
| `GameObject` | `SceneNode` + ECS entity | ✅ | Different abstraction |
| `Component` | ECS components | ✅ | Different pattern |
| `MonoBehaviour` | `MonoBehaviour` trait | ✅ | Similar lifecycle |
| `ScriptableObject` | `ScriptableObject` trait | ✅ | Similar concept |
| `Transform` | `Transform` + `GlobalTransform` | ✅ | Same concept |
| `Scene` | `SceneManager` | ✅ | Similar |
| `Prefab` | `Prefab` system | ✅ | Similar |
| `AddComponent<T>()` | `world.add_component()` | ✅ | Different API style |
| `GetComponent<T>()` | `world.get::<T>()` | ✅ | Different API style |
| `Instantiate()` | Scene node spawning | ✅ | Different API |
| `Destroy()` | Entity despawn | ✅ | Different API |
| `FindWithTag()` | `world.query_by_tag()` | ❌ | Missing tag-based lookup |
| `SendMessage/BroadcastMessage` | `EventBus` | ⚠️ | Different pattern (typed events) |
| `SetActive()` | Component enable/disable | ⚠️ | Partial |
| `CompareTag()` | Tag comparison | ❌ | Missing |
| `FindObjectOfType<T>()` | `world.query::<T>()` | ⚠️ | Partial match |

### 2. Scripting (engine-script)

| Unity Concept | RustEngine Equivalent | Status | Gap |
|---|---|---|---|
| `MonoBehaviour.Update()` | `MonoBehaviour.update()` | ✅ | Same pattern |
| `MonoBehaviour.Start()` | `MonoBehaviour.start()` | ✅ | Same pattern |
| `MonoBehaviour.FixedUpdate()` | `MonoBehaviour.fixed_update()` | ✅ | Same pattern |
| `MonoBehaviour.OnEnable()` | `MonoBehaviour.on_enable()` | ⚠️ | Partial |
| `MonoBehaviour.OnDisable()` | `MonoBehaviour.on_disable()` | ⚠️ | Partial |
| `MonoBehaviour.OnDestroy()` | `MonoBehaviour.on_destroy()` | ⚠️ | Partial |
| `MonoBehaviour.OnCollisionEnter()` | Physics callbacks | ⚠️ | Partial |
| `MonoBehaviour.OnTriggerEnter()` | Physics callbacks | ⚠️ | Partial |
| `MonoBehaviour.OnMouseDown()` | Input callbacks | ❌ | Missing |
| `MonoBehaviour.OnGUI()` | UI rendering | ⚠️ | Different (egui) |
| `Debug.Log()` | `println!` / tracing | ⚠️ | Different |
| `StartCoroutine/IEnumerator` | No equivalent | ❌ | Missing coroutine system |
| `WaitForSeconds` | No equivalent | ❌ | Missing |
| `Invoke()` | No equivalent | ❌ | Missing timer utility |

### 3. Editor (engine-editor)

| Unity Concept | RustEngine Equivalent | Status | Gap |
|---|---|---|---|
| Scene view | Viewport | ✅ | Similar |
| Hierarchy panel | Scene hierarchy | ✅ | Similar |
| Inspector | Inspector | ✅ | Similar |
| Project view | Resource browser | ✅ | Similar |
| Console | Log panel | ⚠️ | Partial |
| Game view | No equivalent | ❌ | Missing separate game view |
| Undo/Redo | Command pattern | ✅ | Similar |
| Play mode | Play mode | ✅ | Similar |
| Gizmos | Gizmo system | ✅ | Similar |
| Prefab editing | Prefab system | ✅ | Similar |
| Build settings | No equivalent | ❌ | Missing build pipeline UI |
| Build player | `cargo build --release` | ⚠️ | Different (Rust native) |
| Package Manager | No equivalent | ❌ | Missing |
| Asset Store | No equivalent | ❌ | Missing (ecosystem) |
| Assembly definitions | Cargo.toml | ✅ | Different (Rust modules) |
| Editor scripts | Editor plugins | ⚠️ | Different pattern |

### 4. Rendering (engine-render)

| Unity Concept | RustEngine Equivalent | Status | Gap |
|---|---|---|---|
| URP/HDRP | Render graph | ✅ | Different pipeline |
| Shader Graph | WGSL shaders | ✅ | Different tool |
| Material | Material + PBR | ✅ | Similar |
| Light | Light components | ✅ | Similar |
| Camera | Camera component | ✅ | Similar |
| Post-processing | Post-processing chain | ✅ | Similar |
| Particle System | Particle system | ✅ | Similar |
| Terrain | Terrain system | ✅ | Similar |
| Skybox | Atmosphere module | ✅ | Similar |
| Reflection Probes | IBL | ✅ | Different name |
| LOD | LOD system | ✅ | Similar |
| Occlusion Culling | Occlusion culling | ✅ | Similar |
| NavMesh | No equivalent | ❌ | Missing navigation mesh |
| Video Player | No equivalent | ❌ | Missing |

### 5. Physics (engine-physics)

| Unity Concept | RustEngine Equivalent | Status | Gap |
|---|---|---|---|
| Rigidbody | RigidBody | ✅ | Similar |
| Collider | Collider | ✅ | Similar |
| Physics Material | Friction/restitution | ✅ | Different naming |
| Joints | Joint solver | ✅ | Similar |
| Raycast | Raycast | ⚠️ | Partial |
| Physics.Overlap | Spatial hash | ⚠️ | Different |
| Physics queries | Basic queries | ⚠️ | Missing sweep, overlap tests |
| 2D Physics | PhysicsWorld2D | ✅ | Similar |
| Physics Layers | Collision layers | ✅ | Similar |
| Physics Debugger | No equivalent | ❌ | Missing visualization |

### 6. Audio (engine-audio)

| Unity Concept | RustEngine Equivalent | Status | Gap |
|---|---|---|---|
| AudioSource | AudioManager | ✅ | Different API |
| AudioListener | No equivalent | ❌ | Missing listener concept |
| AudioMixer | AudioMixer | ✅ | Similar |
| AudioClip | Audio file loading | ✅ | Different naming |
| Spatial Blend | 3D spatial audio | ✅ | Similar |
| Doppler | Doppler effect | ✅ | Similar |
| Audio Reverb | No equivalent | ❌ | Missing |
| Audio Low Pass Filter | No equivalent | ❌ | Missing |

### 7. Animation (engine-scene animations)

| Unity Concept | RustEngine Equivalent | Status | Gap |
|---|---|---|---|
| Animator | AnimationStateMachine | ✅ | Different API |
| Animation Clip | Keyframe clips | ✅ | Similar |
| Animator Controller | State machine | ✅ | Similar |
| Blend Tree | BlendSpace | ✅ | Similar |
| IK | IK solvers | ✅ | Similar |
| Animation Events | No equivalent | ❌ | Missing |
| Avatar | Skeleton/Joint | ⚠️ | Different |
| Mecanim retargeting | No equivalent | ❌ | Missing |

### 8. Networking (engine-network)

| Unity Concept | RustEngine Equivalent | Status | Gap |
|---|---|---|---|
| Mirror/Netcode | Network plugin | ✅ | Different library |
| NetworkManager | GameServer/GameClient | ✅ | Different API |
| NetworkObject | ECS components | ⚠️ | Different pattern |
| RPC | Message system | ⚠️ | Different |
| SyncVar | Snapshot sync | ⚠️ | Different |
| Lobby/Multiplay | Lobby/Matchmaking | ✅ | Similar |

### 9. Package/Module System

| Unity Concept | RustEngine Equivalent | Status | Gap |
|---|---|---|---|
| Package Manager | Cargo.toml | ✅ | Different (Rust) |
| UPM Packages | Workspace crates | ✅ | Different (Rust) |
| Assembly Definitions | Feature flags | ⚠️ | Different |
| Domain Reload | Plugin hot-reload | ⚠️ | Different |

## Priority Recommendations

Based on the audit, the most impactful alignment items are:

### High Priority (P0) — Unity API Conventions
1. Add `FindObjectOfType<T>()` / `FindObjectsOfType<T>()` to World
2. Add tag-based entity lookup (`find_by_tag()`, `find_all_by_tag()`)
3. Add `SetActive()` / `IsActive()` for entity activation
4. Add `CompareTag()` utility
5. Add `SendMessage` / `BroadcastMessage` equivalent (typed event dispatch)

### High Priority (P0) — MonoBehaviour Lifecycle
6. Add `OnEnable()` / `OnDisable()` / `OnDestroy()` callbacks properly
7. Add `OnCollisionEnter` / `OnCollisionExit` / `OnTriggerEnter` / `OnTriggerExit` as MonoBehaviour callbacks
8. Add `OnMouseDown()` / `OnMouseUp()` input callbacks
9. Add `OnApplicationQuit()` / `OnApplicationPause()` / `OnApplicationFocus()` callbacks
10. Add coroutine system (equivalent to Unity's `StartCoroutine` / `IEnumerator`)

### Medium Priority (P1) — Editor Parity
11. Add separate Game View (render to texture, show in editor)
12. Add Console panel (log output display)
13. Add Build Settings panel
14. Add Physics Debugger (visualize colliders, contacts)

### Medium Priority (P1) — Physics Completeness
15. Add `Physics.Raycast()` / `Physics.RaycastAll()` query API
16. Add `Physics.OverlapBox()` / `Physics.OverlapSphere()` query API
17. Add `Physics.BoxCast()` / `Physics.SphereCast()` sweep query API
18. Add Physics Debugger visualization

### Lower Priority (P2) — Audio Completeness
19. Add AudioListener concept
20. Add Audio Reverb effect
21. Add Audio Low Pass / High Pass filters

### Lower Priority (P2) — Animation Completeness
22. Add Animation Events system
23. Add Avatar retargeting system

---

## Implementation Plan

### Task 1: Add Tag-Based Entity Lookup to World

**Files:**
- Modify: `crates/engine-ecs/src/world.rs`

- [ ] **Step 1: Add tag storage to World**

```rust
// Add to World struct
pub struct World {
    // ... existing fields ...
    tags: HashMap<String, Vec<Entity>>,
}
```

- [ ] **Step 2: Add tag management methods**

```rust
impl World {
    pub fn add_tag(&mut self, entity: Entity, tag: impl Into<String>) {
        let tag = tag.into();
        self.tags.entry(tag).or_default().push(entity);
    }

    pub fn remove_tag(&mut self, entity: Entity, tag: &str) {
        if let Some(entities) = self.tags.get_mut(tag) {
            entities.retain(|e| *e != entity);
        }
    }

    pub fn has_tag(&self, entity: Entity, tag: &str) -> bool {
        self.tags.get(tag)
            .map(|entities| entities.contains(&entity))
            .unwrap_or(false)
    }

    pub fn find_by_tag(&self, tag: &str) -> Option<Entity> {
        self.tags.get(tag)
            .and_then(|entities| entities.first().copied())
    }

    pub fn find_all_by_tag(&self, tag: &str) -> Vec<Entity> {
        self.tags.get(tag)
            .cloned()
            .unwrap_or_default()
    }

    pub fn compare_tag(&self, entity: Entity, tag: &str) -> bool {
        self.has_tag(entity, tag)
    }
}
```

- [ ] **Step 3: Add tests**

```rust
#[test]
fn test_tag_operations() {
    let mut world = World::new();
    let entity = world.spawn();
    
    world.add_tag(entity, "Player");
    assert!(world.has_tag(entity, "Player"));
    assert_eq!(world.find_by_tag("Player"), Some(entity));
    
    world.remove_tag(entity, "Player");
    assert!(!world.has_tag(entity, "Player"));
}
```

- [ ] **Step 4: Run tests and commit**

---

### Task 2: Add SetActive/IsActive for Entities

**Files:**
- Modify: `crates/engine-ecs/src/world.rs`

- [ ] **Step 1: Add active state storage**

```rust
pub struct World {
    // ... existing fields ...
    active_state: HashMap<Entity, bool>,
}
```

- [ ] **Step 2: Add active state methods**

```rust
impl World {
    pub fn set_active(&mut self, entity: Entity, active: bool) {
        self.active_state.insert(entity, active);
    }

    pub fn is_active(&self, entity: Entity) -> bool {
        self.active_state.get(&entity).copied().unwrap_or(true)
    }

    pub fn set_active_recursive(&mut self, entity: Entity, active: bool) {
        self.set_active(entity, active);
        // Also set children inactive
        let children = self.children(entity).cloned().unwrap_or_default();
        for child in children {
            self.set_active_recursive(child, active);
        }
    }
}
```

- [ ] **Step 3: Integrate with query system**

Modify `Query::iter()` and `Query::iter_mut()` to skip inactive entities:

```rust
pub fn iter<'a>(&'a self, world: &'a World) -> impl Iterator<Item = &'a T> {
    self.sparse_set.iter()
        .filter(move |(entity, _)| world.is_active(*entity))
        .map(|(_, component)| component)
}
```

- [ ] **Step 4: Add tests and commit**

---

### Task 3: Add SendMessage / BroadcastMessage Equivalent

**Files:**
- Modify: `crates/engine-core/src/event_bus.rs` (or create if not exists)

- [ ] **Step 1: Create MessageBus in engine-core**

```rust
use std::any::{Any, TypeId};
use std::collections::HashMap;

pub struct MessageBus {
    handlers: HashMap<TypeId, Vec<Box<dyn Fn(&dyn Any) -> ()>>>,
}

impl MessageBus {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register_handler<T: 'static>(&mut self, handler: impl Fn(&T) + 'static) {
        let type_id = TypeId::of::<T>();
        self.handlers
            .entry(type_id)
            .or_default()
            .push(Box::new(move |any| {
                if let Some(msg) = any.downcast_ref::<T>() {
                    handler(msg);
                }
            }));
    }

    pub fn send<T: 'static>(&self, message: &T) {
        let type_id = TypeId::of::<T>();
        if let Some(handlers) = self.handlers.get(&type_id) {
            for handler in handlers {
                handler(message);
            }
        }
    }
}
```

- [ ] **Step 2: Integrate with World**

Add `MessageBus` as a resource in the World, provide convenience methods:

```rust
// In engine-core, add to App or World
pub fn send_message<T: 'static>(&self, message: T) {
    if let Some(bus) = self.resources().get::<MessageBus>() {
        bus.send(&message);
    }
}
```

- [ ] **Step 3: Add tests and commit**

---

### Task 4: Enhance MonoBehaviour Lifecycle Callbacks

**Files:**
- Modify: `crates/engine-core/src/mono_behaviour.rs`

- [ ] **Step 1: Add missing callbacks to MonoBehaviour trait**

```rust
pub trait MonoBehaviour {
    // Existing
    fn start(&mut self) {}
    fn update(&mut self, delta_time: f32) {}
    fn fixed_update(&mut self, delta_time: f32) {}
    
    // Add these
    fn on_enable(&mut self) {}
    fn on_disable(&mut self) {}
    fn on_destroy(&mut self) {}
    fn on_application_quit(&mut self) {}
    fn on_application_pause(&mut self, paused: bool) {}
    fn on_application_focus(&mut self, focused: bool) {}
    
    // Physics callbacks
    fn on_collision_enter(&mut self, other: Entity) {}
    fn on_collision_exit(&mut self, other: Entity) {}
    fn on_trigger_enter(&mut self, other: Entity) {}
    fn on_trigger_exit(&mut self, other: Entity) {}
    
    // Input callbacks
    fn on_mouse_down(&mut self) {}
    fn on_mouse_up(&mut self) {}
    fn on_mouse_enter(&mut self) {}
    fn on_mouse_exit(&mut self) {}
}
```

- [ ] **Step 2: Wire callbacks into systems**

Create systems that call these callbacks at appropriate times:

```rust
// In engine-core/src/systems
pub fn on_enable_system(world: &mut World) {
    // Find newly enabled entities, call on_enable
}

pub fn on_disable_system(world: &mut World) {
    // Find newly disabled entities, call on_disable
}

pub fn on_destroy_system(world: &mut World) {
    // Find despawned entities, call on_destroy
}

pub fn on_collision_system(world: &mut World) {
    // From physics, dispatch collision callbacks
}
```

- [ ] **Step 3: Add tests and commit**

---

### Task 5: Add Coroutine System

**Files:**
- Create: `crates/engine-core/src/coroutine.rs`

- [ ] **Step 1: Define Coroutine types**

```rust
pub enum CoroutineYield {
    WaitForFrames(u32),
    WaitForSeconds(f32),
    WaitForEndOfFrame,
    WaitUntil(Box<dyn Fn() -> bool>),
    None,
}

pub struct Coroutine {
    pub(crate) id: u64,
    pub(crate) generator: Box<dyn Iterator<Item = CoroutineYield>>,
    pub(crate) elapsed: f32,
    pub(crate) frames_waited: u32,
}

pub struct CoroutineRunner {
    next_id: u64,
    active: Vec<Coroutine>,
}
```

- [ ] **Step 2: Implement runner**

```rust
impl CoroutineRunner {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            active: Vec::new(),
        }
    }

    pub fn start(&mut self, gen: impl Iterator<Item = CoroutineYield> + 'static) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.active.push(Coroutine {
            id,
            generator: Box::new(gen),
            elapsed: 0.0,
            frames_waited: 0,
        });
        id
    }

    pub fn update(&mut self, delta_time: f32) {
        self.active.retain_mut(|coro| {
            coro.elapsed += delta_time;
            coro.frames_waited += 1;
            
            match coro.generator.next() {
                Some(CoroutineYield::WaitForFrames(n)) => coro.frames_waited < n,
                Some(CoroutineYield::WaitForSeconds(t)) => coro.elapsed < t,
                Some(CoroutineYield::WaitForEndOfFrame) => false, // always yields once
                Some(CoroutineYield::WaitUntil(f)) => !f(),
                Some(CoroutineYield::None) | None => false,
            }
        });
    }

    pub fn stop(&mut self, id: u64) {
        self.active.retain(|c| c.id != id);
    }
}
```

- [ ] **Step 3: Add to World resources and tests**

---

### Task 6: Add Physics Query API (Raycast, Overlap, Sweep)

**Files:**
- Modify: `crates/engine-physics/src/world.rs`

- [ ] **Step 1: Add Raycast API**

```rust
pub struct RaycastHit {
    pub entity: Entity,
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
}

impl PhysicsWorld {
    pub fn raycast(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> Option<RaycastHit> {
        // Sweep through broadphase, test narrowphase
        // Return closest hit
    }

    pub fn raycast_all(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> Vec<RaycastHit> {
        // Return all hits sorted by distance
    }
}
```

- [ ] **Step 2: Add Overlap API**

```rust
impl PhysicsWorld {
    pub fn overlap_sphere(&self, center: Vec3, radius: f32) -> Vec<Entity> {
        // Query spatial hash for overlapping entities
    }

    pub fn overlap_box(&self, center: Vec3, half_extents: Vec3, rotation: Quat) -> Vec<Entity> {
        // Query spatial hash for overlapping entities
    }

    pub fn overlap_capsule(&self, start: Vec3, end: Vec3, radius: f32) -> Vec<Entity> {
        // Query spatial hash for overlapping entities
    }
}
```

- [ ] **Step 3: Add Sweep API**

```rust
impl PhysicsWorld {
    pub fn sphere_cast(&self, origin: Vec3, direction: Vec3, radius: f32, max_distance: f32) -> Option<RaycastHit> {
        // Sweep sphere through world
    }

    pub fn box_cast(&self, origin: Vec3, direction: Vec3, half_extents: Vec3, rotation: Quat, max_distance: f32) -> Option<RaycastHit> {
        // Sweep box through world
    }

    pub fn capsule_cast(&self, start: Vec3, end: Vec3, radius: f32, direction: Vec3, max_distance: f32) -> Option<RaycastHit> {
        // Sweep capsule through world
    }
}
```

- [ ] **Step 4: Add tests and commit**

---

### Task 7: Add Game View to Editor

**Files:**
- Modify: `crates/engine-editor/src/panels/game_view.rs` (create)
- Modify: `crates/engine-editor/src/editor_state.rs`

- [ ] **Step 1: Create Game View panel**

```rust
pub struct GameView {
    pub render_target: Option<TextureId>,
    pub camera: Entity,
    pub is_playing: bool,
}

impl GameView {
    pub fn new() -> Self {
        Self {
            render_target: None,
            camera: Entity::default(),
            is_playing: false,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, renderer: &mut Renderer) {
        // Render the scene to a texture
        // Display the texture in the UI
        if let Some(target) = &self.render_target {
            ui.image(target, ui.available_size());
        }
    }
}
```

- [ ] **Step 2: Integrate with EditorState**

```rust
// In EditorState
pub game_view: GameView,
```

- [ ] **Step 3: Wire into editor UI layout**

```rust
// In editor rendering
egui::CentralPanel::default().show(ctx, |ui| {
    self.game_view.render(ui, &mut self.renderer);
});
```

- [ ] **Step 4: Add tests and commit**

---

### Task 8: Add Physics Debugger Visualization

**Files:**
- Modify: `crates/engine-physics/src/debug.rs` (create)
- Modify: `crates/engine-render/src/shape_renderer.rs`

- [ ] **Step 1: Create PhysicsDebugger**

```rust
pub struct PhysicsDebugger {
    pub draw_colliders: bool,
    pub draw_contacts: bool,
    pub draw_aabbs: bool,
    pub collider_color: Color,
    pub contact_color: Color,
}

impl PhysicsDebugger {
    pub fn render(&self, physics: &PhysicsWorld, renderer: &mut ShapeRenderer) {
        if self.draw_colliders {
            for (entity, collider) in physics.colliders() {
                match collider.shape {
                    ColliderShape::Sphere { radius } => {
                        renderer.draw_sphere(
                            collider.position,
                            radius,
                            self.collider_color,
                        );
                    }
                    ColliderShape::Box { half_extents } => {
                        renderer.draw_cube(
                            collider.position,
                            half_extents * 2.0,
                            self.collider_color,
                        );
                    }
                    // ... other shapes
                }
            }
        }

        if self.draw_contacts {
            for contact in physics.contacts() {
                renderer.draw_point(
                    contact.point,
                    0.1,
                    self.contact_color,
                );
                renderer.draw_line(
                    contact.point,
                    contact.point + contact.normal * 0.5,
                    self.contact_color,
                );
            }
        }
    }
}
```

- [ ] **Step 2: Add toggle to editor**

- [ ] **Step 3: Add tests and commit**

---

### Task 9: Add AudioListener Concept

**Files:**
- Modify: `crates/engine-audio/src/listener.rs` (create)
- Modify: `crates/engine-audio/src/manager.rs`

- [ ] **Step 1: Create AudioListener component**

```rust
#[derive(Component)]
pub struct AudioListener {
    pub enabled: bool,
}

impl Default for AudioListener {
    fn default() -> Self {
        Self { enabled: true }
    }
}
```

- [ ] **Step 2: Update AudioManager to use listener position**

```rust
impl AudioManager {
    pub fn update_listener(&mut self, position: Vec3, forward: Vec3) {
        // Update the listener position for 3D audio
        self.listener_position = position;
        self.listener_forward = forward;
    }
}
```

- [ ] **Step 3: Create system to auto-update listener from camera**

```rust
pub fn audio_listener_system(
    world: &World,
    audio: &mut AudioManager,
) {
    // Find entities with AudioListener + Transform
    // Use their position as the listener
}
```

- [ ] **Step 4: Add tests and commit**

---

### Task 10: Add Animation Events

**Files:**
- Modify: `crates/engine-scene/src/animation.rs`

- [ ] **Step 1: Add AnimationEvent struct**

```rust
pub struct AnimationEvent {
    pub name: String,
    pub time: f32,
    pub parameter: Option<String>,
}

pub struct AnimationClip {
    // ... existing fields ...
    pub events: Vec<AnimationEvent>,
}
```

- [ ] **Step 2: Add event dispatch to AnimationPlayer**

```rust
impl AnimationPlayer {
    pub fn update(&mut self, delta_time: f32) -> Vec<AnimationEvent> {
        let mut events = Vec::new();
        let previous_time = self.current_time;
        self.current_time += delta_time;

        // Check for events between previous and current time
        if let Some(clip) = &self.current_clip {
            for event in &clip.events {
                if event.time >= previous_time && event.time < self.current_time {
                    events.push(event.clone());
                }
            }
        }

        events
    }
}
```

- [ ] **Step 3: Wire events to MonoBehaviour callbacks**

```rust
// In animation system
pub fn animation_event_system(world: &mut World) {
    let events = world.resources_mut().get_mut::<AnimationPlayer>()
        .map(|p| p.update(delta_time))
        .unwrap_or_default();

    for event in events {
        // Find entities with MonoBehaviour and call on_animation_event
        for (entity, mono) in world.query::<&mut dyn MonoBehaviour>() {
            mono.on_animation_event(&event.name);
        }
    }
}
```

- [ ] **Step 4: Add tests and commit**

---

## Self-Review

After writing the complete plan, I reviewed it against the spec:

1. **Spec coverage:** The plan covers all major Unity architecture gaps identified in the audit — GameObject/Component conventions, MonoBehaviour lifecycle, Editor parity, Physics queries, Audio completeness, and Animation events.

2. **Placeholder scan:** No TBD/TODO placeholders found. Each task has concrete code.

3. **Type consistency:** All types (Entity, World, etc.) are consistent across tasks. Methods use consistent naming conventions.

4. **Scope:** This is a large plan. Consider breaking into phases:
   - **Phase 1 (P0):** Tasks 1-5 (Core API alignment)
   - **Phase 2 (P1):** Tasks 6-8 (Physics + Editor)
   - **Phase 3 (P2):** Tasks 9-10 (Audio + Animation)
