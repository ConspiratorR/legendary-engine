# Architecture Overview

RustEngine is a modular game engine built in Rust with 16 crates organized in layers. This document describes the high-level architecture, crate relationships, and data flow.

## Crate Dependency Graph

```mermaid
graph TD
    subgraph Foundation["Foundation Layer"]
        MATH[engine-math<br/>Glam re-exports]
        JOBS[engine-jobs<br/>Thread pool & job graphs]
    end

    subgraph Core["Core Layer"]
        ECS[engine-ecs<br/>Entity Component System]
        WINDOW[engine-window<br/>winit windowing]
        INPUT[engine-input<br/>Keyboard/mouse/action mapping]
        ASSET[engine-asset<br/>Asset loading & management]
    end

    subgraph Systems["Systems Layer"]
        SCENE[engine-scene<br/>Scene graph & hierarchy]
        RENDER[engine-render<br/>wgpu rendering pipeline]
        PHYSICS[engine-physics<br/>Rigid body dynamics]
        AUDIO[engine-audio<br/>rodio audio playback]
        NETWORK[engine-network<br/>Client/server networking]
        UI[engine-ui<br/>egui integration]
        SCRIPT[engine-script<br/>Lua/WASM scripting]
    end

    subgraph App["Application Layer"]
        FRAMEWORK[engine-framework<br/>Game states & flow]
        EDITOR[engine-editor<br/>Scene editor & tools]
        CORE[engine-core<br/>App builder & plugin system]
    end

    ECS --> MATH
    RENDER --> MATH
    RENDER --> ECS
    RENDER --> ASSET
    RENDER --> WINDOW
    SCENE --> ECS
    SCENE --> MATH
    INPUT --> WINDOW
    PHYSICS --> ECS
    PHYSICS --> MATH
    AUDIO --> MATH
    NETWORK --> ECS
    UI --> ECS
    UI --> RENDER
    SCRIPT --> ECS
    SCRIPT --> ASSET

    CORE --> ECS
    CORE --> SCENE
    CORE --> RENDER
    CORE --> INPUT
    CORE --> ASSET
    CORE --> WINDOW
    CORE --> MATH
    CORE --> AUDIO

    FRAMEWORK --> CORE
    EDITOR --> CORE
    EDITOR --> UI
    EDITOR --> SCENE

    ECS -.->|optional| JOBS
```

## Layer Descriptions

### Foundation Layer

| Crate | Purpose | Key Dependencies |
|-------|---------|-----------------|
| **engine-math** | Re-exports `glam` types (`Vec2/3/4`, `Mat4`, `Quat`) with extension traits | `glam` |
| **engine-jobs** | Thread pool, job graphs, task scheduling | `crossbeam`, `rayon` |

### Core Layer

| Crate | Purpose | Key Dependencies |
|-------|---------|-----------------|
| **engine-ecs** | Sparse-set ECS: entities, components, queries, schedules | `rayon`, optional `engine-jobs` |
| **engine-window** | Window creation via winit 0.30 | `winit` |
| **engine-input** | Keyboard/mouse state tracking, action maps, input bindings | `engine-window` |
| **engine-asset** | Asset handles (`Arc` ref-counting), type registry, file watcher, loaders (image, glTF, audio) | `notify`, `image`, `gltf` |

### Systems Layer

| Crate | Purpose | Key Dependencies |
|-------|---------|-----------------|
| **engine-scene** | Scene nodes, parent-child hierarchy, Transform/GlobalTransform sync, prefabs, animation state | `engine-ecs`, `engine-math` |
| **engine-render** | wgpu renderer, render graph, sprite/PBR pipelines, camera, lighting, shadows, particles, tilemap | `wgpu`, `engine-ecs`, `engine-asset` |
| **engine-physics** | Rigid bodies, colliders, collision detection (SAT), contact solving, joints, CCD | `engine-ecs`, `engine-math` |
| **engine-audio** | Audio playback via rodio, mixer buses, spatial audio, streaming | `rodio`, `engine-math` |
| **engine-network** | Message serialization, client/server, authoritative mode, snapshot sync, NAT traversal | `engine-ecs`, `serde` |
| **engine-ui** | egui integration, theming, layout, retained mode widgets, animations | `egui`, `engine-ecs` |
| **engine-script** | Lua (mlua) and WASM scripting, component bridge, hot-reload, event bus | `mlua`, `engine-ecs` |

### Application Layer

| Crate | Purpose | Key Dependencies |
|-------|---------|-----------------|
| **engine-core** | `AppBuilder`, plugin system, time management, config, logging, profiler | All core + systems crates |
| **engine-framework** | Game state stack, standard game flow (title→menu→game→pause→gameover), save system | `engine-core` |
| **engine-editor** | Scene editor UI, hierarchy panel, inspector, gizmos, undo/redo, scene serialization | `engine-core`, `engine-ui`, `engine-scene` |

## Data Flow

### Frame Lifecycle

```mermaid
sequenceDiagram
    participant W as winit EventLoop
    participant A as App
    participant I as Input
    participant E as ECS World
    participant S as Schedule
    participant R as Renderer

    W->>A: AboutToWait
    A->>I: Process input events
    A->>E: Apply commands
    A->>S: Run systems
    S->>E: Update components
    A->>R: Render frame
    R->>R: Execute render graph
    R->>W: Present surface
```

### ECS System Execution

Systems are registered via the plugin system and run in a `Schedule`:

1. **Startup systems** — Run once after app initialization
2. **Pre-update systems** — Input processing, event handling
3. **Update systems** — Game logic, physics, animation
4. **Post-update systems** — Transform sync, camera sort
5. **Render systems** — Collect draw calls, submit to GPU

### Render Pipeline

```mermaid
graph LR
    subgraph RenderGraph["Render Graph"]
        SHADOW[Shadow Pass]
        GBUFFER[G-Buffer Pass]
        LIGHTING[Lighting Pass]
        POST[Post-Processing]
        SPRITE[Sprite Pass]
        FINAL[Final Composite]
    end

    SHADOW --> GBUFFER
    GBUFFER --> LIGHTING
    LIGHTING --> POST
    SPRITE --> FINAL
    POST --> FINAL
```

## Plugin System

The engine uses a plugin architecture for modularity:

```rust
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;

struct MyPlugin;

impl Plugin for MyPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // Register systems, resources, event handlers
        app.add_system(my_system);
        app.insert_resource(MyResource::default());
    }
}
```

Plugins can:
- Register ECS systems (startup, update, render)
- Insert global resources
- Register event handlers
- Configure the renderer
- Add custom asset loaders

## Feature Flags

| Crate | Feature | Description |
|-------|---------|-------------|
| `engine-core` | `audio` (default) | Enable audio system via `engine-audio` |
| `engine-ecs` | `jobs-backend` | Use `engine-jobs` for parallel system execution |

## Cross-Platform Support

The engine supports Windows, macOS, Linux, and Android (experimental) through:
- **wgpu** for cross-platform GPU abstraction
- **winit** for window management
- **rodio** for audio (with platform-specific backends)
- Conditional compilation via `#[cfg(target_os = "...")]` where needed
