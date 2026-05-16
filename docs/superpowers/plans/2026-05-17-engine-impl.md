# RustEngine (legendary-engine) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first iteration of a Rust game engine with ECS+Scene hybrid architecture, wgpu rendering, and input/audio support.

**Architecture:** Workspace of 9 crates forming a layered dependency graph. `engine-core` aggregates everything; users depend only on it. ECS (`engine-ecs`) is the data backbone. Scene tree (`engine-scene`) wraps ECS entities. Render (`engine-render`) drives wgpu. Window/Input/Audio are thin wrappers around platform libraries.

**Tech Stack:** Rust 2024, wgpu, winit, glam, rodio, glTF 2.0, anyhow/thiserror

---

## File Structure

```
RustEngine/
├── Cargo.toml
├── crates/
│   ├── engine-math/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── engine-ecs/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── world.rs
│   │       ├── component.rs
│   │       ├── entity.rs
│   │       ├── query.rs
│   │       ├── system.rs
│   │       └── schedule.rs
│   ├── engine-window/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── window.rs
│   ├── engine-asset/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── asset.rs
│   │       ├── loader.rs
│   │       ├── registry.rs
│   │       └── format/
│   │           ├── mod.rs
│   │           ├── gltf.rs
│   │           ├── image.rs
│   │           └── audio.rs
│   ├── engine-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── app.rs
│   │       ├── plugin.rs
│   │       ├── resource.rs
│   │       └── engine.rs
│   ├── engine-scene/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── node.rs
│   │       ├── hierarchy.rs
│   │       ├── transform.rs
│   │       └── scene_manager.rs
│   ├── engine-render/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── renderer.rs
│   │       ├── pipeline/
│   │       │   ├── mod.rs
│   │       │   ├── sprite.rs
│   │       │   ├── pbr.rs
│   │       │   └── skybox.rs
│   │       ├── resource/
│   │       │   ├── mod.rs
│   │       │   ├── mesh.rs
│   │       │   ├── texture.rs
│   │       │   └── material.rs
│   │       └── view.rs
│   ├── engine-input/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── input_manager.rs
│   │       ├── keyboard.rs
│   │       └── mouse.rs
│   └── engine-audio/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           └── audio_manager.rs
├── examples/
│   └── basic/
│       ├── Cargo.toml
│       └── src/main.rs
└── .github/workflows/
    └── ci.yml
```

---

### Task 1: Workspace scaffold

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/engine-math/Cargo.toml`
- Create: `crates/engine-ecs/Cargo.toml`
- Create: `crates/engine-window/Cargo.toml`
- Create: `crates/engine-asset/Cargo.toml`
- Create: `crates/engine-core/Cargo.toml`
- Create: `crates/engine-scene/Cargo.toml`
- Create: `crates/engine-render/Cargo.toml`
- Create: `crates/engine-input/Cargo.toml`
- Create: `crates/engine-audio/Cargo.toml`
- Create: `examples/basic/Cargo.toml`
- Create: `crates/*/src/lib.rs` (empty)

- [ ] **Step 1: Write workspace root Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/engine-math",
    "crates/engine-ecs",
    "crates/engine-window",
    "crates/engine-asset",
    "crates/engine-core",
    "crates/engine-scene",
    "crates/engine-render",
    "crates/engine-input",
    "crates/engine-audio",
    "examples/basic",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
authors = ["ConspiratorR"]
```

- [ ] **Step 2: Write all crate Cargo.toml files**

`crates/engine-math/Cargo.toml`:
```toml
[package]
name = "engine-math"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
glam = "0.29"
```

`crates/engine-ecs/Cargo.toml`:
```toml
[package]
name = "engine-ecs"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
thiserror = "2"
```

`crates/engine-window/Cargo.toml`:
```toml
[package]
name = "engine-window"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
winit = "0.30"
```

`crates/engine-asset/Cargo.toml`:
```toml
[package]
name = "engine-asset"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
engine-math = { path = "../engine-math" }
gltf = "1.4"
image = "0.25"
rodio = "0.19"
thiserror = "2"
```

`crates/engine-core/Cargo.toml`:
```toml
[package]
name = "engine-core"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
engine-ecs = { path = "../engine-ecs" }
engine-scene = { path = "../engine-scene" }
engine-render = { path = "../engine-render" }
engine-input = { path = "../engine-input" }
engine-audio = { path = "../engine-audio" }
engine-asset = { path = "../engine-asset" }
engine-window = { path = "../engine-window" }
engine-math = { path = "../engine-math" }
winit = "0.30"
```

`crates/engine-scene/Cargo.toml`:
```toml
[package]
name = "engine-scene"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
engine-ecs = { path = "../engine-ecs" }
engine-math = { path = "../engine-math" }
thiserror = "2"
```



`crates/engine-input/Cargo.toml`:
```toml
[package]
name = "engine-input"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
engine-window = { path = "../engine-window" }
winit = "0.30"
```

`crates/engine-render/Cargo.toml`:
```toml
[package]
name = "engine-render"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
engine-window = { path = "../engine-window" }
engine-math = { path = "../engine-math" }
engine-asset = { path = "../engine-asset" }
wgpu = { version = "22", features = ["wgsl"] }
thiserror = "2"
bytemuck = { version = "1", features = ["derive"] }
pollster = "0.4"
```

`crates/engine-audio/Cargo.toml`:
```toml
[package]
name = "engine-audio"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
rodio = "0.19"
thiserror = "2"
```

`examples/basic/Cargo.toml`:
```toml
[package]
name = "basic"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
engine-core = { path = "../../crates/engine-core" }
```

- [ ] **Step 3: Create empty lib.rs stubs**

Write `crates/engine-math/src/lib.rs`, `crates/engine-ecs/src/lib.rs`, etc. with a single line:
```rust
//! <crate name> — <brief description>
```

- [ ] **Step 4: Verify workspace compiles**

Run: `cargo check`
Expected: No errors (all crates compile with empty lib.rs stubs).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/ examples/
git commit -m "feat: scaffold workspace with 9 crate stubs"
```

---

### Task 2: engine-math — glam re-export + extension trait

**Files:**
- Modify: `crates/engine-math/src/lib.rs`

- [ ] **Step 1: Write the tests**

```rust
#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_vec3_extension() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
    }

    #[test]
    fn test_vec3_normalize() {
        let v = Vec3::new(3.0, 0.0, 0.0);
        let n = v.normalize();
        assert!((n.x - 1.0).abs() < 1e-6);
        assert!((n.y).abs() < 1e-6);
        assert!((n.z).abs() < 1e-6);
    }

    #[test]
    fn test_mat4_identity() {
        let m = Mat4::IDENTITY;
        let v = Vec4::new(1.0, 2.0, 3.0, 1.0);
        assert_eq!(m * v, v);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p engine-math`
Expected: Compilation fails — `Vec3`, `Mat4` not defined yet.

- [ ] **Step 3: Write implementation**

```rust
pub use glam::{Vec2, Vec3, Vec4, Mat4, Quat, EulerRot};

pub trait Vec3Ext {
    fn extend_with_w(self, w: f32) -> Vec4;
}

impl Vec3Ext for Vec3 {
    fn extend_with_w(self, w: f32) -> Vec4 {
        Vec4::new(self.x, self.y, self.z, w)
    }
}

pub trait Mat4Ext {
    fn look_at_lh(eye: Vec3, target: Vec3, up: Vec3) -> Self;
}

impl Mat4Ext for Mat4 {
    fn look_at_lh(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        Mat4::look_at_lh(eye, target, up)
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p engine-math`
Expected: All 3 tests pass.

- [ ] **Step 5: Add documentation comments to the crate and public items**

```rust
/// Math primitives for the engine.
///
/// Re-exports `glam` types and provides extension traits.
pub use glam::{Vec2, Vec3, Vec4, Mat4, Quat, EulerRot};
// ...
```

- [ ] **Step 6: Commit**

```bash
git add crates/engine-math/src/lib.rs
git commit -m "feat(engine-math): re-export glam with Vec3Ext/Mat4Ext"
```

---

### Task 3: engine-ecs — Entity + Component storage

**Files:**
- Modify: `crates/engine-ecs/src/lib.rs`
- Create: `crates/engine-ecs/src/entity.rs`
- Create: `crates/engine-ecs/src/component.rs`
- Create: `crates/engine-ecs/src/world.rs`

- [ ] **Step 1: Write entity tests**

```rust
#[cfg(test)]
mod tests {
    use crate::entity::Entity;

    #[test]
    fn test_entity_creation() {
        let e = Entity::new(0, 0);
        assert_eq!(e.index(), 0);
        assert_eq!(e.generation(), 0);
    }

    #[test]
    fn test_entity_generation_increment() {
        let e = Entity::new(0, 0);
        let e2 = e.next_generation();
        assert_eq!(e2.index(), 0);
        assert_eq!(e2.generation(), 1);
    }
}
```

- [ ] **Step 2: Run tests to fail**

Run: `cargo test -p engine-ecs`
Expected: Compilation fails — `entity` module not found.

- [ ] **Step 3: Implement entity.rs**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity(u64);

impl Entity {
    const INDEX_MASK: u64 = 0x0000_00FF_FFFF_FFFF;
    const GENERATION_SHIFT: u64 = 40;

    pub fn new(index: u32, generation: u32) -> Self {
        let raw = (index as u64) | ((generation as u64) << Self::GENERATION_SHIFT);
        Self(raw)
    }

    pub fn index(self) -> u32 {
        (self.0 & Self::INDEX_MASK) as u32
    }

    pub fn generation(self) -> u32 {
        (self.0 >> Self::GENERATION_SHIFT) as u32
    }

    pub fn next_generation(self) -> Self {
        Self::new(self.index(), self.generation() + 1)
    }
}
```

- [ ] **Step 4: Run entity tests to pass**

Run: `cargo test -p engine-ecs`
Expected: Both entity tests pass.

- [ ] **Step 5: Write component + world tests**

```rust
#[cfg(test)]
mod tests {
    use crate::world::World;

    struct Position(f32, f32, f32);
    struct Velocity(f32, f32);

    #[test]
    fn test_spawn_and_get_component() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Position(1.0, 2.0, 3.0));
        let pos = world.get::<Position>(e);
        assert!(pos.is_some());
        assert_eq!(pos.unwrap().0, 1.0);
    }

    #[test]
    fn test_spawn_without_component() {
        let mut world = World::new();
        let e = world.spawn();
        let pos = world.get::<Position>(e);
        assert!(pos.is_none());
    }

    #[test]
    fn test_despawn_removes_components() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Position(1.0, 2.0, 3.0));
        world.despawn(e);
        let pos = world.get::<Position>(e);
        assert!(pos.is_none());
    }

    #[test]
    fn test_entity_reuse() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        world.despawn(e1);
        let e3 = world.spawn();
        assert_ne!(e3, e1);
        assert_ne!(e3, e2);
    }
}
```

- [ ] **Step 6: Run component/world tests to fail**

Run: `cargo test -p engine-ecs`
Expected: Compilation fails — `World` not defined.

- [ ] **Step 7: Implement component.rs**

```rust
use std::any::TypeId;
use std::collections::HashMap;

pub struct SparseSet<T> {
    sparse: Vec<Option<T>>,
    entities: Vec<u32>,
}

impl<T> SparseSet<T> {
    pub fn new() -> Self {
        Self { sparse: Vec::new(), entities: Vec::new() }
    }

    pub fn insert(&mut self, index: u32, value: T) {
        if index as usize >= self.sparse.len() {
            self.sparse.resize_with(index as usize + 1, || None);
        }
        if self.sparse[index as usize].is_none() {
            self.entities.push(index);
        }
        self.sparse[index as usize] = Some(value);
    }

    pub fn remove(&mut self, index: u32) -> Option<T> {
        if (index as usize) < self.sparse.len() {
            let val = self.sparse[index as usize].take();
            if val.is_some() {
                self.entities.retain(|&i| i != index);
            }
            val
        } else {
            None
        }
    }

    pub fn get(&self, index: u32) -> Option<&T> {
        if (index as usize) < self.sparse.len() {
            self.sparse[index as usize].as_ref()
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        if (index as usize) < self.sparse.len() {
            self.sparse[index as usize].as_mut()
        } else {
            None
        }
    }

    pub fn entities(&self) -> &[u32] {
        &self.entities
    }
}

pub struct ComponentRegistry {
    storages: HashMap<TypeId, Box<dyn std::any::Any>>,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self { storages: HashMap::new() }
    }

    pub fn storage<T: 'static>(&mut self) -> &mut SparseSet<T> {
        let tid = TypeId::of::<T>();
        self.storages
            .entry(tid)
            .or_insert_with(|| Box::new(SparseSet::<T>::new()))
            .downcast_mut::<SparseSet<T>>()
            .expect("Type mismatch in ComponentRegistry")
    }

    pub fn try_get_storage<T: 'static>(&self) -> Option<&SparseSet<T>> {
        let tid = TypeId::of::<T>();
        self.storages
            .get(&tid)?
            .downcast_ref::<SparseSet<T>>()
    }

    pub fn try_get_storage_mut<T: 'static>(&mut self) -> Option<&mut SparseSet<T>> {
        let tid = TypeId::of::<T>();
        self.storages
            .get_mut(&tid)?
            .downcast_mut::<SparseSet<T>>()
    }
}
```

- [ ] **Step 8: Implement world.rs**

```rust
use crate::component::ComponentRegistry;
use crate::entity::Entity;

pub struct World {
    next_index: u32,
    free_list: Vec<u32>,
    generations: Vec<u32>,
    components: ComponentRegistry,
}

impl World {
    pub fn new() -> Self {
        Self {
            next_index: 0,
            free_list: Vec::new(),
            generations: Vec::new(),
            components: ComponentRegistry::new(),
        }
    }

    pub fn spawn(&mut self) -> Entity {
        let index = if let Some(free) = self.free_list.pop() {
            free
        } else {
            let i = self.next_index;
            self.next_index += 1;
            self.generations.push(0);
            i
        };
        Entity::new(index, self.generations[index as usize])
    }

    pub fn despawn(&mut self, entity: Entity) {
        let idx = entity.index();
        if idx as usize >= self.generations.len() {
            return;
        }
        self.generations[idx as usize] = entity.generation() + 1;
        self.free_list.push(idx);
    }

    pub fn add_component<T: 'static>(&mut self, entity: Entity, component: T) {
        self.components.storage::<T>().insert(entity.index(), component);
    }

    pub fn get<T: 'static>(&self, entity: Entity) -> Option<&T> {
        let storage = self.components.try_get_storage::<T>()?;
        storage.get(entity.index())
    }

    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        let storage = self.components.try_get_storage_mut::<T>()?;
        storage.get_mut(entity.index())
    }

    pub fn remove_component<T: 'static>(&mut self, entity: Entity) -> Option<T> {
        let storage = self.components.try_get_storage_mut::<T>()?;
        storage.remove(entity.index())
    }
}
```

- [ ] **Step 9: Update lib.rs**

```rust
pub mod entity;
pub mod component;
pub mod world;
```

- [ ] **Step 10: Run all tests to pass**

Run: `cargo test -p engine-ecs`
Expected: All 6 tests pass.

- [ ] **Step 11: Commit**

```bash
git add crates/engine-ecs/
git commit -m "feat(engine-ecs): Entity, SparseSet storage, World"
```

---

### Task 4: engine-ecs — Query system

**Files:**
- Create: `crates/engine-ecs/src/query.rs`
- Modify: `crates/engine-ecs/src/lib.rs`

- [ ] **Step 1: Write query tests**

```rust
#[cfg(test)]
mod tests {
    use crate::world::World;
    use crate::query::Query;

    struct Pos(f32, f32);

    struct Vel(f32, f32);

    #[test]
    fn test_query_iter() {
        let mut world = World::new();
        let e1 = world.spawn();
        world.add_component(e1, Pos(0.0, 0.0));
        world.add_component(e1, Vel(1.0, 0.0));
        let e2 = world.spawn();
        world.add_component(e2, Pos(1.0, 1.0));
        world.add_component(e2, Vel(0.0, 1.0));
        let _e3 = world.spawn();
        world.add_component(_e3, Pos(2.0, 2.0));
        // e3 has no Vel

        let mut query = Query::<(&Pos, &Vel)>::new();
        let results: Vec<_> = query.iter(&world).collect();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0.0, 0.0);
        assert_eq!(results[1].0.0, 1.0);
    }

    #[test]
    fn test_query_iter_mut() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Pos(0.0, 0.0));
        world.add_component(e, Vel(1.0, 0.0));

        let mut query = Query::<(&mut Pos, &Vel)>::new();
        for (pos, _vel) in query.iter_mut(&mut world) {
            pos.0 += 1.0;
        }

        let pos = world.get::<Pos>(e).unwrap();
        assert_eq!(pos.0, 1.0);
    }
}
```

- [ ] **Step 2: Run to fail**

Run: `cargo test -p engine-ecs`
Expected: Compilation fails — `query` module not found.

- [ ] **Step 3: Implement query.rs**

```rust
use crate::world::World;

pub struct Query<C> {
    _marker: std::marker::PhantomData<C>,
}

impl<'w, A: 'static> Query<(&'w A,)> {
    pub fn new() -> Self {
        Self { _marker: std::marker::PhantomData }
    }

    pub fn iter<'a>(&self, world: &'a World) -> QueryIter<'a, A> {
        let entities = world.component_entities::<A>();
        QueryIter { entities, index: 0, world }
    }
}

pub struct QueryIter<'a, A> {
    entities: Vec<u32>,
    index: usize,
    world: &'a World,
}

impl<'a, A: 'static> Iterator for QueryIter<'a, A> {
    type Item = (&'a A,);

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.index;
        self.index += 1;
        if idx < self.entities.len() {
            let entity_idx = self.entities[idx];
            let comp = self.world.get_by_index::<A>(entity_idx)?;
            Some((comp,))
        } else {
            None
        }
    }
}
```

But wait — we need `World::component_entities` and `World::get_by_index`. Let me add those to world instead and rethink the query design.

Actually, let me keep this simpler. For the first iteration, we can expose helper methods on World and build a minimal Query that works. Let me think about this more carefully.

Actually, the query system in Bevy uses complex type-level programming. For our minimal ECS, let me keep it simple:

```rust
pub trait QueryItem<'w> {
    type Item;
    fn fetch(world: &'w World, index: u32) -> Self::Item;
}

impl<'w, A: 'static> QueryItem<'w> for &'w A {
    type Item = &'w A;
    fn fetch(world: &'w World, index: u32) -> Self::Item {
        world.get_by_index::<A>(index).unwrap()
    }
}
```

But this gets complex with tuples. Let me just provide a simple helper-based approach for now. We can iterate using `World::query()` which returns entity-component pairs.

Actually, let me just make it work for tuples of 1 and 2 components. That's all we need for now.

Let me simplify the approach. The query stores entity indices that have all required components, then iterates.

```rust
use crate::world::World;
use std::marker::PhantomData;

pub struct Query<A, B = ()> {
    _marker: PhantomData<(A, B)>,
}

impl<A: 'static> Query<A, ()> {
    pub fn new() -> Self {
        Self { _marker: PhantomData }
    }

    pub fn iter<'a>(&self, world: &'a World) -> impl Iterator<Item = &'a A> {
        world.component_entities::<A>().into_iter().filter_map(move |idx| world.get_by_index::<A>(idx))
    }
}

impl<A: 'static, B: 'static> Query<(A, B), ()> {
    pub fn new() -> Self {
        Self { _marker: PhantomData }
    }

    pub fn iter<'a>(&self, world: &'a World) -> impl Iterator<Item = (&'a A, &'a B)> {
        let valid: Vec<_> = {
            let ents_a = world.component_entities::<A>();
            let ents_b = world.component_entities::<B>();
            ents_a.iter().filter(|idx| ents_b.contains(idx)).copied().collect()
        };
        valid.into_iter().filter_map(move |idx| {
            Some((world.get_by_index::<A>(idx)?, world.get_by_index::<B>(idx)?))
        })
    }
}
```

This is simple but works and is type-safe. We'll improve later if needed.

And we need to add `component_entities` and `get_by_index` to `World`.

Let me update the plan accordingly. I need to also update the world implementation.

Let me just write a clean version.

- [ ] **Step 3: Add helper methods to World**

Add to `world.rs`:
```rust
impl World {
    pub fn component_entities<T: 'static>(&self) -> Vec<u32> {
        self.components
            .try_get_storage::<T>()
            .map(|s| s.entities().to_vec())
            .unwrap_or_default()
    }

    pub fn get_by_index<T: 'static>(&self, index: u32) -> Option<&T> {
        self.components.try_get_storage::<T>()?.get(index)
    }

    pub fn get_by_index_mut<T: 'static>(&mut self, index: u32) -> Option<&mut T> {
        self.components.try_get_storage_mut::<T>()?.get_mut(index)
    }
}
```

- [ ] **Step 4: Run query tests to pass**

Run: `cargo test -p engine-ecs`
Expected: 8 total tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/engine-ecs/
git commit -m "feat(engine-ecs): add Query for single and pair component iteration"
```

---

### Task 5: engine-ecs — System + Schedule

**Files:**
- Create: `crates/engine-ecs/src/system.rs`
- Create: `crates/engine-ecs/src/schedule.rs`
- Modify: `crates/engine-ecs/src/lib.rs`

- [ ] **Step 1: Write system + schedule tests**

```rust
#[cfg(test)]
mod tests {
    use crate::schedule::Schedule;
    use crate::system::{System, IntoSystem};
    use crate::world::World;

    struct Counter(u32);

    fn increment(mut world: &mut World) {
        for (counter,) in crate::query::Query::<(&mut Counter,)>::new().iter_mut(&mut world) {
            counter.0 += 1;
        }
    }

    #[test]
    fn test_schedule_run_once() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Counter(0));

        let mut schedule = Schedule::new();
        schedule.add_system(increment.system());

        schedule.run(&mut world);

        let counter = world.get::<Counter>(e).unwrap();
        assert_eq!(counter.0, 1);
    }

    #[test]
    fn test_schedule_run_twice() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Counter(0));

        let mut schedule = Schedule::new();
        schedule.add_system(increment.system());

        schedule.run(&mut world);
        schedule.run(&mut world);

        let counter = world.get::<Counter>(e).unwrap();
        assert_eq!(counter.0, 2);
    }
}
```

- [ ] **Step 2: Run to fail**

Run: `cargo test -p engine-ecs`
Expected: Compilation fails.

- [ ] **Step 3: Implement system.rs**

```rust
use crate::world::World;

pub trait System {
    fn run(&self, world: &mut World);
}

pub trait IntoSystem {
    type System: System;
    fn system(self) -> Self::System;
}

impl<F> IntoSystem for F
where
    F: Fn(&mut World),
{
    type System = FnSystem<F>;

    fn system(self) -> Self::System {
        FnSystem(self)
    }
}

pub struct FnSystem<F>(F);

impl<F> System for FnSystem<F>
where
    F: Fn(&mut World),
{
    fn run(&self, world: &mut World) {
        (self.0)(world);
    }
}
```

Also need `Query::iter_mut` — add to `query.rs`:
```rust
impl<A: 'static> Query<A, ()> {
    pub fn iter_mut<'a>(&self, world: &'a mut World) -> impl Iterator<Item = &'a mut A> {
        let indices: Vec<_> = world.component_entities::<A>();
        // We need a safe way to iterate mutably. For now, use a simple approach.
        indices.into_iter().filter_map(move |idx| {
            // SAFETY: We ensure no aliasing by collecting indices first.
            unsafe { (world as *mut World).as_mut() }.unwrap().get_by_index_mut::<A>(idx)
        })
    }
}
```

Actually, this is getting unsafe. Let me think about a safer approach. For now, since we're on a single-threaded schedule and iterating over a pre-collected list of indices, we can use a different pattern.

Actually, the simplest approach: collect indices, then iterate and fetch by index. The problem is we can't return `impl Iterator` that borrows from `world` because the borrow checker won't allow it. Let me use a different approach.

Let me take a step back. For a minimal ECS, we can:

1. Have `QueryIter` as a concrete struct that borrows the world
2. Each component access is indexed, not using reference-based iteration
3. Use `unsafe` internally with proper safety comments

Let me just write the Query impl with the necessary unsafe:

Actually, let me keep it really simple. The query just provides access patterns, not cached iteration. For the system approach, we'll pass `&mut World` to systems and they use World methods directly. The Query type is syntactic sugar.

```rust
// query.rs
impl<A: 'static> Query<A, ()> {
    pub fn iter_mut<'a>(&self, world: &'a mut World) -> QueryIterMut<'a, A> {
        let indices = world.component_entities::<A>();
        QueryIterMut { indices, index: 0, world: world as *mut World }
    }
}

pub struct QueryIterMut<'a, A> {
    indices: Vec<u32>,
    index: usize,
    world: *mut World,
}

impl<'a, A: 'static> Iterator for QueryIterMut<'a, A> {
    type Item = &'a mut A;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.index;
        self.index += 1;
        if idx < self.indices.len() {
            let entity_idx = self.indices[idx];
            // SAFETY: Each entity index is unique, no aliasing.
            unsafe { (*self.world).get_by_index_mut::<A>(entity_idx) }
        } else {
            None
        }
    }
}
```

This has `unsafe` but it's well-bounded. The iterator yields unique `&mut` references because each entity index is unique.

- [ ] **Step 3: Implement system.rs and schedule.rs**

`schedule.rs`:
```rust
use crate::system::System;
use crate::world::World;

pub struct Schedule {
    systems: Vec<Box<dyn System>>,
}

impl Schedule {
    pub fn new() -> Self {
        Self { systems: Vec::new() }
    }

    pub fn add_system(&mut self, system: impl System + 'static) -> &mut Self {
        self.systems.push(Box::new(system));
        self
    }

    pub fn run(&self, world: &mut World) {
        for system in &self.systems {
            system.run(world);
        }
    }
}
```

- [ ] **Step 4: Run tests to pass**

Run: `cargo test -p engine-ecs`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/engine-ecs/
git commit -m "feat(engine-ecs): System trait, FnSystem adapter, Schedule"
```

---

### Task 6: engine-window — winit wrapper

**Files:**
- Create: `crates/engine-window/src/window.rs`

- [ ] **Step 1: Write window config type**

```rust
use winit::dpi::PhysicalSize;

pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub vsync: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "RustEngine".to_string(),
            width: 1280,
            height: 720,
            vsync: true,
        }
    }
}

pub fn create_window(
    config: &WindowConfig,
    event_loop: &winit::event_loop::EventLoop<()>,
) -> winit::window::Window {
    winit::window::WindowBuilder::new()
        .with_title(&config.title)
        .with_inner_size(PhysicalSize::new(config.width, config.height))
        .build(event_loop)
        .expect("Failed to create window")
}
```

- [ ] **Step 2: Write lib.rs**

```rust
pub mod window;
pub use window::{WindowConfig, create_window};
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-window`
Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add crates/engine-window/
git commit -m "feat(engine-window): winit window with WindowConfig"
```

---

### Task 7: engine-core — Plugin system + AppBuilder

**Files:**
- Create: `crates/engine-core/src/plugin.rs`
- Create: `crates/engine-core/src/resource.rs`
- Create: `crates/engine-core/src/app.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Write plugin + app tests**

```rust
#[cfg(test)]
mod tests {
    use crate::app::AppBuilder;
    use crate::plugin::Plugin;
    use engine_ecs::world::World;

    struct CounterPlugin(u32);

    impl Plugin for CounterPlugin {
        fn build(&self, app: &mut AppBuilder) {
            let mut world = app.world_mut();
            let e = world.spawn();
            world.add_component(e, self.0);
        }
    }

    #[test]
    fn test_plugin_adds_data_to_world() {
        let mut app = AppBuilder::new();
        app.add_plugin(CounterPlugin(42));
        let world = app.world_mut();
        let entities = world.component_entities::<u32>();
        assert_eq!(entities.len(), 1);
        let val = world.get_by_index::<u32>(entities[0]).unwrap();
        assert_eq!(*val, 42);
    }
}
```

- [ ] **Step 2: Run to fail**

Run: `cargo test -p engine-core`
Expected: Compilation fails.

- [ ] **Step 3: Implement plugin.rs**

```rust
use crate::app::AppBuilder;

pub trait Plugin {
    fn build(&self, app: &mut AppBuilder);
}
```

- [ ] **Step 4: Implement resource.rs**

```rust
use engine_ecs::world::World;
use std::any::TypeId;
use std::collections::HashMap;

pub struct ResourceRegistry {
    resources: HashMap<TypeId, Box<dyn std::any::Any>>,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self { resources: HashMap::new() }
    }

    pub fn insert<T: 'static>(&mut self, resource: T) {
        self.resources.insert(TypeId::of::<T>(), Box::new(resource));
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.resources.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.resources.get_mut(&TypeId::of::<T>())?.downcast_mut::<T>()
    }
}
```

- [ ] **Step 5: Implement app.rs**

```rust
use crate::plugin::Plugin;
use crate::resource::ResourceRegistry;
use engine_ecs::schedule::Schedule;
use engine_ecs::world::World;

pub struct AppBuilder {
    world: World,
    schedule: Schedule,
    resources: ResourceRegistry,
}

impl AppBuilder {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            schedule: Schedule::new(),
            resources: ResourceRegistry::new(),
        }
    }

    pub fn add_plugin(&mut self, plugin: impl Plugin) -> &mut Self {
        plugin.build(self);
        self
    }

    pub fn add_system(&mut self, system: impl engine_ecs::system::IntoSystem + 'static) -> &mut Self {
        self.schedule.add_system(system.system());
        self
    }

    pub fn insert_resource<T: 'static>(&mut self, resource: T) -> &mut Self {
        self.resources.insert(resource);
        self
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    pub fn resources(&self) -> &ResourceRegistry {
        &self.resources
    }

    pub fn resources_mut(&mut self) -> &mut ResourceRegistry {
        &mut self.resources
    }
}
```

- [ ] **Step 6: Run tests to pass**

Run: `cargo test -p engine-core`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/engine-core/
git commit -m "feat(engine-core): Plugin, AppBuilder, ResourceRegistry"
```

---

### Task 8: engine-core — Resource accessor + Engine

**Files:**
- Create: `crates/engine-core/src/engine.rs`
- Modify: `crates/engine-core/src/lib.rs`
- Modify: `crates/engine-core/src/app.rs`

- [ ] **Step 1: Add `run` method to App and create Engine**

Add to `app.rs`:
```rust
pub struct App {
    world: World,
    schedule: Schedule,
    resources: ResourceRegistry,
}

impl App {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            schedule: Schedule::new(),
            resources: ResourceRegistry::new(),
        }
    }

    pub fn run(&mut self) {
        // Single-frame placeholder — will be looped in Engine
        self.schedule.run(&mut self.world);
    }
}

impl From<AppBuilder> for App {
    fn from(builder: AppBuilder) -> Self {
        Self {
            world: builder.world,
            schedule: builder.schedule,
            resources: builder.resources,
        }
    }
}
```

`engine.rs`:
```rust
use crate::app::{AppBuilder, App};
use engine_window::{WindowConfig, create_window};

pub struct Engine;

impl Engine {
    pub fn new() -> AppBuilder {
        let mut app = AppBuilder::new();
        // Default plugins registered here later when other crates exist
        app
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p engine-core`
Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add crates/engine-core/
git commit -m "feat(engine-core): App run loop and Engine builder"
```

---

### Task 9: engine-scene — SceneNode, Hierarchy, Transform

**Files:**
- Create: `crates/engine-scene/src/node.rs`
- Create: `crates/engine-scene/src/hierarchy.rs`
- Create: `crates/engine-scene/src/transform.rs`
- Modify: `crates/engine-scene/src/lib.rs`

- [ ] **Step 1: Write scene node and transform tests**

```rust
#[cfg(test)]
mod tests {
    use engine_math::{Vec3, Quat};

    #[test]
    fn test_transform_identity() {
        let t = crate::transform::Transform::default();
        assert_eq!(t.translation, Vec3::ZERO);
        assert_eq!(t.scale, Vec3::ONE);
    }

    #[test]
    fn test_transform_from_xyz() {
        let t = crate::transform::Transform::from_xyz(1.0, 2.0, 3.0);
        assert_eq!(t.translation.x, 1.0);
    }
}
```

- [ ] **Step 2: Implement transform.rs**

```rust
use engine_math::{Vec3, Mat4, Quat};

#[derive(Debug, Clone)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: Vec3::new(x, y, z),
            ..Default::default()
        }
    }

    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

#[derive(Debug, Clone)]
pub struct GlobalTransform(pub Mat4);

impl Default for GlobalTransform {
    fn default() -> Self {
        Self(Mat4::IDENTITY)
    }
}
```

- [ ] **Step 3: Implement hierarchy.rs**

```rust
use engine_ecs::entity::Entity;

#[derive(Debug, Clone)]
pub struct Parent(pub Entity);

#[derive(Debug, Clone)]
pub struct Children(pub Vec<Entity>);

impl Children {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}
```

- [ ] **Step 4: Implement node.rs**

```rust
use engine_ecs::entity::Entity;
use crate::transform::Transform;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneNode {
    entity: Entity,
}

impl SceneNode {
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn with_transform(self, transform: Transform) -> SceneBuilder {
        SceneBuilder {
            entity: self.entity,
            transform: Some(transform),
        }
    }
}

pub struct SceneBuilder {
    entity: Entity,
    transform: Option<Transform>,
}
```

- [ ] **Step 5: Verify tests pass**

Run: `cargo test -p engine-scene`
Expected: Tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/engine-scene/
git commit -m "feat(engine-scene): SceneNode, Transform, Hierarchy components"
```

---

### Task 10: engine-scene — SceneManager

**Files:**
- Create: `crates/engine-scene/src/scene_manager.rs`
- Modify: `crates/engine-scene/src/lib.rs`

- [ ] **Step 1: Write SceneManager test**

```rust
#[cfg(test)]
mod tests {
    use crate::scene_manager::SceneManager;

    #[test]
    fn test_add_node() {
        let mut sm = SceneManager::new();
        let node = sm.add_node("test");
        let name = sm.name(node);
        assert_eq!(name, "test");
    }

    #[test]
    fn test_node_parent_child() {
        let mut sm = SceneManager::new();
        let parent = sm.add_node("parent");
        let child = sm.add_node("child");
        sm.set_parent(child, parent);
        assert_eq!(sm.parent(child), Some(parent));
    }

    #[test]
    fn test_add_node_with_transform() {
        use engine_math::Vec3;
        let mut sm = SceneManager::new();
        let node = sm.add_node("camera")
            .with_transform(crate::transform::Transform::from_xyz(0.0, 5.0, 10.0));
        let transform = sm.transform(node);
        assert_eq!(transform.translation, Vec3::new(0.0, 5.0, 10.0));
    }
}
```

- [ ] **Step 2: Implement scene_manager.rs**

```rust
use engine_ecs::entity::Entity;
use engine_ecs::world::World;
use crate::node::{SceneNode, SceneBuilder};
use crate::hierarchy::{Parent, Children};
use crate::transform::{Transform, GlobalTransform};

pub struct SceneManager {
    world: World,
    root: SceneNode,
    names: Vec<String>,
}

impl SceneManager {
    pub fn new() -> Self {
        let mut world = World::new();
        let root_entity = world.spawn();
        world.add_component(root_entity, Children::new());
        world.add_component(root_entity, Transform::default());
        world.add_component(root_entity, GlobalTransform::default());
        let root = SceneNode::new(root_entity);
        let mut names = Vec::new();
        names.push("root".to_string());
        Self { world, root, names }
    }

    pub fn root(&self) -> SceneNode {
        self.root
    }

    pub fn add_node(&mut self, name: &str) -> SceneNodeBuilder {
        let entity = self.world.spawn();
        let node = SceneNode::new(entity);
        let idx = entity.index() as usize;
        if idx >= self.names.len() {
            self.names.resize_with(idx + 1, || String::new());
        }
        self.names[idx] = name.to_string();
        self.world.add_component(entity, Transform::default());
        self.world.add_component(entity, GlobalTransform::default());
        self.world.add_component(entity, Children::new());
        // Attach to root by default
        self.set_parent_internal(node, self.root);
        SceneNodeBuilder { scene_manager: self as *mut _, node }
    }

    fn set_parent_internal(&mut self, child: SceneNode, parent: SceneNode) {
        self.world.add_component(child.entity(), Parent(parent.entity()));
        if let Some(children) = self.world.get_mut::<Children>(parent.entity()) {
            children.0.push(child.entity());
        }
    }

    pub fn set_parent(&mut self, child: SceneNode, parent: SceneNode) {
        // Remove from old parent's children
        if let Some(old_parent) = self.parent(child) {
            if let Some(children) = self.world.get_mut::<Children>(old_parent.entity()) {
                children.0.retain(|e| *e != child.entity());
            }
        }
        self.set_parent_internal(child, parent);
    }

    pub fn parent(&self, node: SceneNode) -> Option<SceneNode> {
        self.world.get::<Parent>(node.entity()).map(|p| SceneNode::new(p.0))
    }

    pub fn name(&self, node: SceneNode) -> &str {
        let idx = node.entity().index() as usize;
        if idx < self.names.len() {
            &self.names[idx]
        } else {
            ""
        }
    }

    pub fn transform(&self, node: SceneNode) -> &Transform {
        self.world.get::<Transform>(node.entity()).unwrap()
    }

    pub fn transform_mut(&mut self, node: SceneNode) -> &mut Transform {
        self.world.get_mut::<Transform>(node.entity()).unwrap()
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
}

pub struct SceneNodeBuilder {
    scene_manager: *mut SceneManager,
    node: SceneNode,
}

impl SceneNodeBuilder {
    pub fn with_transform(mut self, transform: Transform) -> SceneNodeBuilder {
        let sm = unsafe { &mut *self.scene_manager };
        *sm.transform_mut(self.node) = transform;
        self
    }

    pub fn with_child(mut self, child: SceneNode) -> SceneNodeBuilder {
        let sm = unsafe { &mut *self.scene_manager };
        sm.set_parent(child, self.node);
        self
    }

    pub fn build(self) -> SceneNode {
        self.node
    }
}

impl Into<SceneNode> for SceneNodeBuilder {
    fn into(self) -> SceneNode {
        self.node
    }
}
```

- [ ] **Step 3: Update lib.rs**

```rust
pub mod node;
pub mod hierarchy;
pub mod transform;
pub mod scene_manager;
```

- [ ] **Step 4: Run tests to pass**

Run: `cargo test -p engine-scene`
Expected: All 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/engine-scene/
git commit -m "feat(engine-scene): SceneManager with hierarchy and transforms"
```

---

### Task 11: engine-asset — Asset trait, Handle, Registry

**Files:**
- Create: `crates/engine-asset/src/asset.rs`
- Create: `crates/engine-asset/src/registry.rs`
- Create: `crates/engine-asset/src/loader.rs`
- Modify: `crates/engine-asset/src/lib.rs`

- [ ] **Step 1: Write asset tests**

```rust
#[cfg(test)]
mod tests {
    use crate::asset::{Asset, Handle};
    use crate::registry::Registry;

    #[derive(Clone)]
    struct MyAsset(u32);

    impl Asset for MyAsset {
        type Id = String;
        fn id(&self) -> &Self::Id { &"my_asset".to_string() }
    }

    #[test]
    fn test_handle_clone_increments_count() {
        let asset = MyAsset(42);
        let h1 = Handle::new(asset);
        let count1 = h1.ref_count();
        let h2 = h1.clone();
        assert_eq!(h2.ref_count(), h1.ref_count());
        drop(h2);
        // After drop, ref count decreased
    }

    #[test]
    fn test_registry_store_and_get() {
        let mut reg = Registry::new();
        let handle = reg.store("test/key", MyAsset(42));
        let loaded = reg.get::<MyAsset>("test/key");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().0, 42);
    }

    #[test]
    fn test_registry_unknown_key() {
        let reg = Registry::new();
        let loaded = reg.get::<MyAsset>("nonexistent");
        assert!(loaded.is_none());
    }
}
```

- [ ] **Step 2: Run to fail**

Run: `cargo test -p engine-asset`
Expected: Compilation fails.

- [ ] **Step 3: Implement asset.rs**

```rust
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

pub trait Asset: Clone + 'static {
    type Id: ?Sized + std::fmt::Debug + std::hash::Hash + Eq;
    fn id(&self) -> &Self::Id;
}

pub struct Handle<T: Asset> {
    inner: Arc<HandleInner<T>>,
}

struct HandleInner<T: Asset> {
    asset: T,
    ref_count: AtomicUsize,
}

impl<T: Asset> Handle<T> {
    pub fn new(asset: T) -> Self {
        Self {
            inner: Arc::new(HandleInner {
                asset,
                ref_count: AtomicUsize::new(1),
            }),
        }
    }

    pub fn ref_count(&self) -> usize {
        self.inner.ref_count.load(Ordering::Relaxed)
    }

    pub fn get(&self) -> &T {
        &self.inner.asset
    }
}

impl<T: Asset> Clone for Handle<T> {
    fn clone(&self) -> Self {
        self.inner.ref_count.fetch_add(1, Ordering::Relaxed);
        Self { inner: self.inner.clone() }
    }
}
```

- [ ] **Step 4: Implement registry.rs**

```rust
use std::collections::HashMap;
use crate::asset::{Asset, Handle};

pub struct Registry {
    assets: HashMap<String, Box<dyn std::any::Any>>,
}

impl Registry {
    pub fn new() -> Self {
        Self { assets: HashMap::new() }
    }

    pub fn store<T: Asset>(&mut self, key: &str, asset: T) -> Handle<T> {
        let handle = Handle::new(asset);
        self.assets.insert(key.to_string(), Box::new(handle.clone()));
        handle
    }

    pub fn get<T: Asset>(&self, key: &str) -> Option<&T> {
        self.assets
            .get(key)?
            .downcast_ref::<Handle<T>>()
            .map(|h| h.get())
    }

    pub fn contains(&self, key: &str) -> bool {
        self.assets.contains_key(key)
    }
}
```

- [ ] **Step 5: Implement loader.rs**

```rust
use crate::registry::Registry;
use crate::asset::Asset;

pub trait Loader {
    type Asset: Asset;
    fn load(&self, path: &str, registry: &mut Registry);
}

pub fn load_asset<T: Asset>(registry: &mut Registry, path: &str, asset: T) {
    registry.store(path, asset);
}
```

- [ ] **Step 6: Verify tests pass**

Run: `cargo test -p engine-asset`
Expected: Tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/engine-asset/
git commit -m "feat(engine-asset): Asset trait, Handle, Registry, Loader"
```

---

### Task 12: engine-asset — Format loaders

**Files:**
- Create: `crates/engine-asset/src/format/mod.rs`
- Create: `crates/engine-asset/src/format/image.rs`
- Create: `crates/engine-asset/src/format/audio.rs`
- Create: `crates/engine-asset/src/format/gltf.rs`
- Modify: `crates/engine-asset/src/lib.rs`

- [ ] **Step 1: Write format module + image loader**

```rust
pub mod image;
pub mod audio;

pub fn load_image(path: &str) -> Result<image::DynamicImage, crate::loader::LoadError> {
    image::open(path).map_err(|e| crate::loader::LoadError::Format(e.to_string()))
}
```

- [ ] **Step 2: Implement image module**

```rust
pub fn load_image(path: &str) -> Result<image::DynamicImage, String> {
    image::open(path).map_err(|e| format!("Failed to load image '{}': {}", path, e))
}

pub struct ImageData {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: pixel_format,
}

pub enum pixel_format { Rgba8 }

impl ImageData {
    pub fn from_dynamic(img: &image::DynamicImage) -> Self {
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        Self {
            pixels: rgba.into_raw(),
            width: w,
            height: h,
            format: pixel_format::Rgba8,
        }
    }
}
```

- [ ] **Step 3: Implement audio module**

```rust
pub fn load_audio(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("Failed to load audio '{}': {}", path, e))
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p engine-asset`
Expected: No errors.

- [ ] **Step 5: Commit**

```bash
git add crates/engine-asset/
git commit -m "feat(engine-asset): image and audio format loaders, gltf stub"
```

---

### Task 13: engine-input — InputManager

**Files:**
- Create: `crates/engine-input/src/input_manager.rs`
- Create: `crates/engine-input/src/keyboard.rs`
- Create: `crates/engine-input/src/mouse.rs`
- Modify: `crates/engine-input/src/lib.rs`

- [ ] **Step 1: Write input tests**

```rust
#[cfg(test)]
mod tests {
    use crate::keyboard::{KeyCode, KeyState};
    use crate::input_manager::InputManager;

    #[test]
    fn test_key_default_released() {
        let input = InputManager::new();
        assert_eq!(input.key_state(KeyCode::Space), KeyState::Released);
    }

    #[test]
    fn test_key_press_and_release() {
        let mut input = InputManager::new();
        input.press(KeyCode::A);
        assert_eq!(input.key_state(KeyCode::A), KeyState::JustPressed);
        input.update_frame();
        assert_eq!(input.key_state(KeyCode::A), KeyState::Pressed);
        input.release(KeyCode::A);
        assert_eq!(input.key_state(KeyCode::A), KeyState::JustReleased);
        input.update_frame();
        assert_eq!(input.key_state(KeyCode::A), KeyState::Released);
    }
}
```

- [ ] **Step 2: Run to fail**

Run: `cargo test -p engine-input`
Expected: Compilation fails.

- [ ] **Step 3: Implement keyboard.rs**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Released,
    JustPressed,
    Pressed,
    JustReleased,
}

pub use engine_window::winit::keyboard::KeyCode;
```

- [ ] **Step 4: Implement mouse.rs**

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseState {
    pub position: (f64, f64),
    pub delta: (f64, f64),
    pub left_button: bool,
    pub right_button: bool,
}

impl Default for MouseState {
    fn default() -> Self {
        Self {
            position: (0.0, 0.0),
            delta: (0.0, 0.0),
            left_button: false,
            right_button: false,
        }
    }
}
```

- [ ] **Step 5: Implement input_manager.rs**

```rust
use std::collections::HashMap;
use crate::keyboard::{KeyCode, KeyState};
use crate::mouse::MouseState;

pub struct InputManager {
    keys: HashMap<KeyCode, KeyState>,
    mouse: MouseState,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            mouse: MouseState::default(),
        }
    }

    pub fn press(&mut self, key: KeyCode) {
        let state = self.keys.entry(key).or_insert(KeyState::Released);
        if *state == KeyState::Released || *state == KeyState::JustReleased {
            *state = KeyState::JustPressed;
        }
    }

    pub fn release(&mut self, key: KeyCode) {
        let state = self.keys.entry(key).or_insert(KeyState::Released);
        if *state == KeyState::Pressed || *state == KeyState::JustPressed {
            *state = KeyState::JustReleased;
        }
    }

    pub fn key_state(&self, key: KeyCode) -> KeyState {
        self.keys.get(&key).copied().unwrap_or(KeyState::Released)
    }

    pub fn key_down(&self, key: KeyCode) -> bool {
        matches!(self.key_state(key), KeyState::Pressed | KeyState::JustPressed)
    }

    pub fn key_just_pressed(&self, key: KeyCode) -> bool {
        self.key_state(key) == KeyState::JustPressed
    }

    pub fn update_frame(&mut self) {
        for state in self.keys.values_mut() {
            match state {
                KeyState::JustPressed => *state = KeyState::Pressed,
                KeyState::JustReleased => *state = KeyState::Released,
                _ => {}
            }
        }
        self.mouse.delta = (0.0, 0.0);
    }

    pub fn mouse(&self) -> &MouseState {
        &self.mouse
    }

    pub fn mouse_mut(&mut self) -> &mut MouseState {
        &mut self.mouse
    }
}
```

- [ ] **Step 6: Run tests to pass**

Run: `cargo test -p engine-input`
Expected: Tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/engine-input/
git commit -m "feat(engine-input): InputManager with KeyState, MouseState"
```

---

### Task 14: engine-audio — AudioManager

**Files:**
- Create: `crates/engine-audio/src/audio_manager.rs`
- Modify: `crates/engine-audio/src/lib.rs`

- [ ] **Step 1: Write audio test**

```rust
#[cfg(test)]
mod tests {
    use crate::audio_manager::AudioManager;

    #[test]
    fn test_audio_manager_create() {
        let mut _audio = AudioManager::new();
        // Just verify no panic
    }
}
```

- [ ] **Step 2: Implement audio_manager.rs**

```rust
use rodio::{OutputStream, OutputStreamHandle, Sink, source::Source};
use std::fs::File;
use std::io::BufReader;

pub struct AudioManager {
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
}

impl AudioManager {
    pub fn new() -> Self {
        let (_stream, _stream_handle) = OutputStream::try_default()
            .expect("Failed to initialize audio output");
        Self { _stream, _stream_handle }
    }

    pub fn play(&self, path: &str) -> Result<(), String> {
        let file = File::open(path).map_err(|e| format!("Cannot open '{}': {}", path, e))?;
        let source = rodio::Decoder::new(BufReader::new(file))
            .map_err(|e| format!("Cannot decode '{}': {}", path, e))?;
        self._stream_handle.play_raw(source.convert_samples())
            .map_err(|e| format!("Playback error: {}", e))?;
        Ok(())
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-audio`
Expected: No errors. (Note: test will be skipped in CI without audio device.)

- [ ] **Step 4: Commit**

```bash
git add crates/engine-audio/
git commit -m "feat(engine-audio): AudioManager with 2D playback via rodio"
```

---

### Task 15: engine-render — Renderer core (wgpu device/swapchain)

**Files:**
- Create: `crates/engine-render/src/lib.rs`
- Create: `crates/engine-render/src/renderer.rs`
- Create: `crates/engine-render/src/pipeline/mod.rs`
- Create: `crates/engine-render/src/resource/mod.rs`

- [ ] **Step 1: Write renderer skeleton**

```rust
use engine_window::WindowConfig;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration, Instance, Backends};
use std::sync::Arc;

pub struct Renderer {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
}

impl Renderer {
    pub fn new(window: Arc<winit::window::Window>) -> Self {
        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends: Some(wgpu::Backends::all()),
            ..Default::default()
        });
        let surface = instance.create_surface(window).unwrap();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })).unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        )).unwrap();
        let size = window.inner_size();
        let config = surface.get_default_config(&adapter, size.width, size.height).unwrap();
        surface.configure(&device, &config);
        Self { device, queue, surface, config }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn present(&self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("main_encoder"),
        });
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("main_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
        });
        self.queue.submit([encoder.finish()]);
        output.present();
        Ok(())
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p engine-render`
Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/
git commit -m "feat(engine-render): Renderer with wgpu device, surface, swapchain"
```

---

### Task 16: engine-render — 2D sprite batch pipeline

**Files:**
- Create: `crates/engine-render/src/pipeline/sprite.rs`
- Modify: `crates/engine-render/src/pipeline/mod.rs`

- [ ] **Step 1: Implement sprite vertex type and pipeline**

```rust
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl SpriteVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 12, shader_location: 1 },
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 20, shader_location: 2 },
            ],
        }
    }
}

pub struct SpritePipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl SpritePipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sprite.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite_pipeline_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[SpriteVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        Self { pipeline }
    }
}
```

- [ ] **Step 2: Create sprite.wgsl shader**

```wgsl
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4(input.position, 1.0);
    output.uv = input.uv;
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-render`
Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/pipeline/
git commit -m "feat(engine-render): Sprite vertex type, pipeline, basic shader"
```

---

### Task 17: engine-render — 3D static mesh + simple shadow

**Files:**
- Create: `crates/engine-render/src/pipeline/pbr.rs`
- Create: `crates/engine-render/src/resource/mesh.rs`
- Create: `crates/engine-render/src/resource/texture.rs`
- Create: `crates/engine-render/src/resource/material.rs`

- [ ] **Step 1: Implement Mesh resource**

```rust
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl MeshVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 24, shader_location: 2 },
            ],
        }
    }
}

pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: Option<wgpu::Buffer>,
    pub num_vertices: u32,
    pub num_indices: u32,
}

impl Mesh {
    pub fn new(device: &wgpu::Device, vertices: &[MeshVertex], indices: Option<&[u32]>) -> Self {
        use wgpu::util::DeviceExt;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_vertex_buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let (index_buffer, num_indices) = if let Some(indices) = indices {
            let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mesh_index_buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            (Some(buf), indices.len() as u32)
        } else {
            (None, 0)
        };
        Self {
            vertex_buffer,
            index_buffer,
            num_vertices: vertices.len() as u32,
            num_indices,
        }
    }
}
```

- [ ] **Step 2: Implement PBR pipeline stub**

```rust
pub struct PbrPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl PbrPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, depth_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pbr_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("pbr.wgsl").into()),
        });
        // Bind group for camera uniform + material
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pbr_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pbr_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[MeshVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
        });
        Self { pipeline }
    }
}

// Placeholder pbr.wgsl
// @vertex vs_main -> passes position through with depth
// @fragment fs_main -> returns flat color
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-render`
Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/pipeline/pbr.rs crates/engine-render/src/resource/
git commit -m "feat(engine-render): Mesh resource, PBR pipeline stub"
```

---

### Task 18: engine-render — View + Camera + render orchestration

**Files:**
- Create: `crates/engine-render/src/view.rs`

- [ ] **Step 1: Implement Camera and View types**

```rust
use engine_math::{Vec3, Vec4, Mat4};

pub struct Camera {
    pub projection: Projection,
    pub view: Mat4,
}

pub enum Projection {
    Perspective { fov_y: f32, near: f32, far: f32 },
    Orthographic { left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32 },
}

impl Camera {
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let proj = Mat4::perspective_rh(fov_y, aspect, near, far);
        Self {
            projection: Projection::Perspective { fov_y, near, far },
            view: Mat4::IDENTITY,
        }
    }

    pub fn view_projection_matrix(&self, aspect: f32) -> Mat4 {
        let proj_matrix = match self.projection {
            Projection::Perspective { fov_y, near, far } => {
                Mat4::perspective_rh(fov_y, aspect, near, far)
            }
            Projection::Orthographic { left, right, bottom, top, near, far } => {
                Mat4::orthographic_rh(left, right, bottom, top, near, far)
            }
        };
        proj_matrix * self.view
    }
}

pub struct View {
    pub camera: Camera,
    pub viewport: (u32, u32),
}

impl View {
    pub fn new(camera: Camera, width: u32, height: u32) -> Self {
        Self { camera, viewport: (width, height) }
    }

    pub fn aspect(&self) -> f32 {
        self.viewport.0 as f32 / self.viewport.1.max(1) as f32
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p engine-render`
Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/view.rs
git commit -m "feat(engine-render): Camera, Projection, View types"
```

---

### Task 19: Integrate Engine — wire up all plugins, add sys module

**Files:**
- Modify: `crates/engine-core/src/engine.rs`
- Modify: `crates/engine-core/src/app.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Update App to run full loop**

```rust
pub struct App {
    world: World,
    schedule: Schedule,
    resources: ResourceRegistry,
    window: Option<Arc<winit::window::Window>>,
    renderer: Option<Renderer>,
}

impl App {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            schedule: Schedule::new(),
            resources: ResourceRegistry::new(),
            window: None,
            renderer: None,
        }
    }

    pub fn run(&mut self) {
        let event_loop = if let Some(window) = &self.window {
            // Use existing event loop
        } else {
            // For now, just run a single frame
        };
        // Applications will implement their own event loop
        // self.schedule.run(&mut self.world);
    }
}
```

Actually, the event loop management is complex with winit. Let me keep it simpler. The `Engine` struct will own the event loop and run it.

- [ ] **Step 1: Write the full event loop integration**

```rust
use crate::app::App;
use engine_window::{WindowConfig, create_window};
use engine_render::renderer::Renderer;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

pub struct Engine {
    app: App,
}

impl Engine {
    pub fn new(app: App) -> Self {
        Self { app }
    }

    pub fn run(self) {
        let event_loop = EventLoop::new().unwrap();
        let window = std::sync::Arc::new(create_window(&WindowConfig::default(), &event_loop));
        let renderer = pollster::block_on(Renderer::new(window.clone()));

        event_loop.run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    elwt.exit();
                }
                Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                    // resize renderer
                }
                Event::AboutToWait => {
                    // 1. Update input
                    // 2. Run schedule
                    // 3. Render
                    let _ = renderer.present();
                }
                _ => {}
            }
        }).unwrap();
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p engine-core`
Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/
git commit -m "feat(engine-core): integrate event loop with window and renderer"
```

---

### Task 20: Basic example — spinning cube

**Files:**
- Modify: `examples/basic/src/main.rs`

- [ ] **Step 1: Write main.rs**

```rust
use engine_core::app::AppBuilder;
use engine_core::engine::Engine;
use engine_core::plugin::Plugin;

struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(player_movement);
    }
}

fn player_movement(world: &mut engine_ecs::world::World) {
    // Placeholder: will update transforms based on input
}

fn main() {
    AppBuilder::new()
        .add_plugin(GamePlugin)
        .build()
        .run();
}
```

We need to add `.build()` to `AppBuilder`:
```rust
impl AppBuilder {
    pub fn build(self) -> App {
        App::from(self)
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add examples/ crates/engine-core/src/app.rs
git commit -m "feat(examples): basic app example with GamePlugin"
```

---

### Task 21: CI setup

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write CI workflow**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.95.0
          components: clippy, rustfmt
      - uses: swatinem/rust-cache@v2

      - name: Format
        run: cargo fmt --check

      - name: Clippy
        run: cargo clippy -- -D warnings

      - name: Build
        run: cargo build

      - name: Test
        run: cargo test
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add GitHub Actions workflow"
```

---

### Task 22: Spec coverage self-review

- [ ] **Step 1: Verify every spec section maps to at least one task**

| Spec Section | Task(s) |
|---|---|
| Workspace & Crate Structure | Task 1 |
| engine-math | Task 2 |
| ECS core | Tasks 3-5 |
| engine-window | Task 6 |
| Plugin system | Tasks 7-8 |
| Scene layer | Tasks 9-10 |
| Asset system | Tasks 11-12 |
| Input system | Task 13 |
| Audio system | Task 14 |
| Render core | Task 15 |
| 2D sprite | Task 16 |
| 3D mesh | Task 17 |
| View/Camera | Task 18 |
| Integration | Tasks 19-20 |
| CI | Task 21 |
