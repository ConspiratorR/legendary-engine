# Architecture Documentation Update Design

## Overview

Update `docs/architecture.md` to reflect the Unity-like API additions and enhanced plugin system in RustEngine.

## Key Changes

### 1. New Module Structure in engine-core

**Unity-like API Modules:**
- `gameobject` - GameObject struct and Component trait
- `monobehaviour` - MonoBehaviour trait with lifecycle callbacks
- `monobehaviour_runner` - System for running MonoBehaviour updates
- `event` - Type-safe EventBus for decoupled communication
- `events` - Built-in event types (Collision, Trigger, Mouse, etc.)
- `player_loop` - Phase-based execution system (Unity-like PlayerLoop)

**Supporting Modules:**
- `hierarchy` - Transform synchronization utilities
- `scriptable_object` - Base class for data assets
- `asset_database` - Centralized asset management
- `asset_handle` - Generic asset handles with type safety
- `serialization` - Scene and prefab serialization
- `prefab` - Prefab instantiation and management
- `undo` - Undo/redo system for editor operations

### 2. Dependency Layer Updates

**engine-core new dependencies:**
- `libloading` - Dynamic plugin loading
- `serde`, `serde_json` - Serialization support
- `engine-audio` - Optional (feature-gated)

**No layer changes** - All crates remain in their original layers.

### 3. Data Flow Updates

**New Frame Lifecycle (PlayerLoop-based):**
```
Initialization → PreFixedUpdate → FixedUpdate → PostFixedUpdate →
PreUpdate → Update → PostUpdate → PreLateUpdate → LateUpdate →
PostLateUpdate → Render → AfterRender → Cleanup
```

**Event System Flow:**
```
EventBus manages type-safe event dispatch:
- MonoBehaviour lifecycle callbacks (OnCollisionEnter, OnTriggerExit, etc.)
- GameObject.SendMessage() for decoupled communication
- Context.events for global event broadcasting
```

### 4. Plugin System Updates

**Static Plugins:**
```rust
impl Plugin for MyPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(my_system);
        app.insert_resource(MyResource::default());
    }
}
```

**Dynamic Plugins:**
- Plugin manifests (`plugin.json`) with version compatibility
- Shared library loading via `libloading`
- Plugin registry and loader for managing installations
- Runtime loading without recompilation

### 5. Unity-like API Integration

**Core Concepts:**
- **GameObject**: Entity with name, tag, layer, and component list
- **Component**: Trait for data containers with lifecycle callbacks
- **MonoBehaviour**: Script with Update, FixedUpdate, LateUpdate, etc.
- **Transform**: Local/world transform synchronization
- **PlayerLoop**: Phase-based execution matching Unity's execution order

## Documentation Structure

1. **Overview** - Add note about Unity-like API
2. **Validated Dependency Layers** - Keep existing (no changes)
3. **Crate Dependency Graph** - Update Mermaid diagram with new modules
4. **Layer Descriptions** - Update engine-core table with new modules
5. **Data Flow** - Replace with PlayerLoop-based execution flow
6. **Plugin System** - Add dynamic plugin section
7. **Unity-like API** - New section explaining core concepts
8. **Feature Flags** - Update with new features
9. **Cross-Platform Support** - Keep existing

## Minimal Examples

Include one example for each major concept:
1. Creating a GameObject with components
2. Implementing a MonoBehaviour
3. Using the EventBus
4. Loading a dynamic plugin

## Success Criteria

- [x] All new modules documented
- [x] Dependency layers verified (no changes needed)
- [x] Data flow reflects PlayerLoop execution
- [x] Plugin system includes dynamic plugins
- [x] Unity-like API concepts explained
- [x] Minimal examples provided
- [x] No contradictions with existing code