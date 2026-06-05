# Phase 1: Cross-Cutting Foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** Fix known test failures, unify error handling across all 17 crates, and establish module dependency rules.

**Architecture:** Bottom-up approach starting with leaf crates. Each crate gets an error.rs with thiserror-derived error types, then unwrap()/expect() calls are audited and replaced. Finally, dependency rules are validated and documented.

**Tech Stack:** Rust 2024 edition, thiserror, anyhow, cargo test, cargo clippy

---

### Task 1: Fix engine-asset test failure (missing tempfile dev-dep)

**Files:**
- Modify: crates/engine-asset/Cargo.toml

- [ ] **Step 1: Add tempfile dev-dependency**

Open crates/engine-asset/Cargo.toml and add under [dev-dependencies]:

`	oml
[dev-dependencies]
tempfile = "3"
`

- [ ] **Step 2: Verify tests pass**

Run: cargo test -p engine-asset
Expected: All tests PASS

- [ ] **Step 3: Commit**

`ash
git add crates/engine-asset/Cargo.toml
git commit -m "fix(asset): add tempfile dev-dependency for tests"
`

---

### Task 2: Fix engine-core example KeyCode variants

**Files:**
- Modify: crates/engine-core/examples/*.rs (any files using outdated KeyCode)

- [ ] **Step 1: Find outdated KeyCode usage**

Run: g "KeyCode::" crates/engine-core/examples/
Identify any KeyCode variants that don't match current winit 0.30 API.

- [ ] **Step 2: Update KeyCode variants**

Common winit 0.30 changes:
- KeyCode::W -> KeyCode::KeyW
- KeyCode::A -> KeyCode::KeyA
- KeyCode::S -> KeyCode::KeyS
- KeyCode::D -> KeyCode::KeyD
- KeyCode::Escape -> KeyCode::Escape (unchanged)
- KeyCode::Space -> KeyCode::Space (unchanged)

Update all affected example files.

- [ ] **Step 3: Verify examples compile**

Run: cargo build --examples -p engine-core
Expected: All examples compile without errors

- [ ] **Step 4: Commit**

`ash
git add crates/engine-core/examples/
git commit -m "fix(core): update KeyCode variants for winit 0.30"
`

---

### Task 3: Verify all tests pass after fixes

**Files:**
- None (verification only)

- [ ] **Step 1: Run full test suite**

Run: cargo test --all
Expected: All tests PASS, 0 failures

- [ ] **Step 2: Run clippy**

Run: cargo clippy --all
Expected: No errors (warnings acceptable for now)

---

### Task 4: Create error types for leaf crates (Layer 0)

**Files:**
- Create: crates/engine-math/src/error.rs
- Create: crates/engine-jobs/src/error.rs
- Create: crates/engine-window/src/error.rs
- Modify: crates/engine-math/src/lib.rs
- Modify: crates/engine-jobs/src/lib.rs
- Modify: crates/engine-window/src/lib.rs

- [ ] **Step 1: Create engine-math error type**

Create crates/engine-math/src/error.rs:

`ust
use thiserror::Error;

/// Errors that can occur in the math module.
#[derive(Error, Debug)]
pub enum MathError {
    #[error("Invalid vector length: {0}")]
    InvalidLength(usize),

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Invalid quaternion: {reason}")]
    InvalidQuaternion { reason: String },

    #[error("Matrix is not invertible")]
    NotInvertible,
}
`

- [ ] **Step 2: Register math error module**

Add to crates/engine-math/src/lib.rs:

`ust
pub mod error;
pub use error::MathError;
`

- [ ] **Step 3: Create engine-jobs error type**

Create crates/engine-jobs/src/error.rs:

`ust
use thiserror::Error;

/// Errors that can occur in the jobs/tasks module.
#[derive(Error, Debug)]
pub enum JobsError {
    #[error("Task pool shutdown")]
    Shutdown,

    #[error("Task panicked: {0}")]
    TaskPanicked(String),

    #[error("Invalid thread count: {0}")]
    InvalidThreadCount(usize),

    #[error("Task timeout after {0}ms")]
    Timeout(u64),
}
`

- [ ] **Step 4: Register jobs error module**

Add to crates/engine-jobs/src/lib.rs:

`ust
pub mod error;
pub use error::JobsError;
`

- [ ] **Step 5: Create engine-window error type**

Create crates/engine-window/src/error.rs:

`ust
use thiserror::Error;

/// Errors that can occur in the window module.
#[derive(Error, Debug)]
pub enum WindowError {
    #[error("Failed to create window: {reason}")]
    CreationFailed { reason: String },

    #[error("Window not found")]
    NotFound,

    #[error("Invalid window size: {width}x{height}")]
    InvalidSize { width: u32, height: u32 },

    #[error("Platform error: {0}")]
    Platform(String),
}
`

- [ ] **Step 6: Register window error module**

Add to crates/engine-window/src/lib.rs:

`ust
pub mod error;
pub use error::WindowError;
`

- [ ] **Step 7: Verify compilation**

Run: cargo build -p engine-math -p engine-jobs -p engine-window
Expected: Compiles without errors

- [ ] **Step 8: Commit**

`ash
git add crates/engine-math/src/error.rs crates/engine-math/src/lib.rs
git add crates/engine-jobs/src/error.rs crates/engine-jobs/src/lib.rs
git add crates/engine-window/src/error.rs crates/engine-window/src/lib.rs
git commit -m "feat: add error types for leaf crates (math, jobs, window)"
`

---

### Task 5: Create error types for Layer 1 crates

**Files:**
- Create: crates/engine-audio/src/error.rs
- Create: crates/engine-asset/src/error.rs
- Create: crates/engine-ecs/src/error.rs
- Modify: crates/engine-audio/src/lib.rs
- Modify: crates/engine-asset/src/lib.rs
- Modify: crates/engine-ecs/src/lib.rs

- [ ] **Step 1: Create engine-audio error type**

Create crates/engine-audio/src/error.rs:

`ust
use thiserror::Error;

/// Errors that can occur in the audio module.
#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Failed to decode audio: {0}")]
    DecodeError(String),

    #[error("Audio device not found")]
    DeviceNotFound,

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Playback error: {0}")]
    PlaybackError(String),

    #[error("Stream error: {0}")]
    StreamError(String),
}
`

- [ ] **Step 2: Register audio error module**

Add to crates/engine-audio/src/lib.rs:

`ust
pub mod error;
pub use error::AudioError;
`

- [ ] **Step 3: Create engine-asset error type**

Create crates/engine-asset/src/error.rs:

`ust
use thiserror::Error;

/// Errors that can occur in the asset module.
#[derive(Error, Debug)]
pub enum AssetError {
    #[error("Asset not found: {path}")]
    NotFound { path: String },

    #[error("Failed to load asset: {path}, reason: {reason}")]
    LoadFailed { path: String, reason: String },

    #[error("Unsupported asset type: {0}")]
    UnsupportedType(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid asset handle")]
    InvalidHandle,
}
`

- [ ] **Step 4: Register asset error module**

Add to crates/engine-asset/src/lib.rs:

`ust
pub mod error;
pub use error::AssetError;
`

- [ ] **Step 5: Create engine-ecs error type**

Create crates/engine-ecs/src/error.rs:

`ust
use thiserror::Error;

/// Errors that can occur in the ECS module.
#[derive(Error, Debug)]
pub enum EcsError {
    #[error("Entity not found: {0:?}")]
    EntityNotFound(crate::entity::Entity),

    #[error("Component not registered: {0}")]
    ComponentNotRegistered(String),

    #[error("Duplicate component")]
    DuplicateComponent,

    #[error("World is locked for modification")]
    WorldLocked,

    #[error("Invalid archetype")]
    InvalidArchetype,

    #[error("Query error: {0}")]
    QueryError(String),
}
`

- [ ] **Step 6: Register ECS error module**

Add to crates/engine-ecs/src/lib.rs:

`ust
pub mod error;
pub use error::EcsError;
`

- [ ] **Step 7: Verify compilation**

Run: cargo build -p engine-audio -p engine-asset -p engine-ecs
Expected: Compiles without errors

- [ ] **Step 8: Commit**

`ash
git add crates/engine-audio/src/error.rs crates/engine-audio/src/lib.rs
git add crates/engine-asset/src/error.rs crates/engine-asset/src/lib.rs
git add crates/engine-ecs/src/error.rs crates/engine-ecs/src/lib.rs
git commit -m "feat: add error types for Layer 1 crates (audio, asset, ecs)"
`

---

### Task 6: Create error types for Layer 2 crates

**Files:**
- Create: crates/engine-scene/src/error.rs
- Create: crates/engine-input/src/error.rs
- Modify: crates/engine-scene/src/lib.rs
- Modify: crates/engine-input/src/lib.rs

- [ ] **Step 1: Create engine-scene error type**

Create crates/engine-scene/src/error.rs:

`ust
use thiserror::Error;

/// Errors that can occur in the scene module.
#[derive(Error, Debug)]
pub enum SceneError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Circular dependency detected")]
    CircularDependency,

    #[error("Invalid parent: {0}")]
    InvalidParent(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),
}
`

- [ ] **Step 2: Register scene error module**

Add to crates/engine-scene/src/lib.rs:

`ust
pub mod error;
pub use error::SceneError;
`

- [ ] **Step 3: Create engine-input error type**

Create crates/engine-input/src/error.rs:

`ust
use thiserror::Error;

/// Errors that can occur in the input module.
#[derive(Error, Debug)]
pub enum InputError {
    #[error("Action not found: {0}")]
    ActionNotFound(String),

    #[error("Invalid binding: {0}")]
    InvalidBinding(String),

    #[error("Duplicate action: {0}")]
    DuplicateAction(String),
}
`

- [ ] **Step 4: Register input error module**

Add to crates/engine-input/src/lib.rs:

`ust
pub mod error;
pub use error::InputError;
`

- [ ] **Step 5: Verify compilation**

Run: cargo build -p engine-scene -p engine-input
Expected: Compiles without errors

- [ ] **Step 6: Commit**

`ash
git add crates/engine-scene/src/error.rs crates/engine-scene/src/lib.rs
git add crates/engine-input/src/error.rs crates/engine-input/src/lib.rs
git commit -m "feat: add error types for Layer 2 crates (scene, input)"
`

---

### Task 7: Create error types for Layer 3-5 crates

**Files:**
- Create: crates/engine-render/src/error.rs
- Create: crates/engine-core/src/error.rs
- Create: crates/engine-framework/src/error.rs
- Create: crates/engine-physics/src/error.rs
- Create: crates/engine-network/src/error.rs
- Create: crates/engine-script/src/error.rs
- Create: crates/engine-ui/src/error.rs
- Create: crates/engine-terrain/src/error.rs
- Modify: Each crate's lib.rs

- [ ] **Step 1: Create engine-render error type**

Create crates/engine-render/src/error.rs:

`ust
use thiserror::Error;

/// Errors that can occur in the render module.
#[derive(Error, Debug)]
pub enum RenderError {
    #[error("GPU initialization failed: {0}")]
    GpuInitFailed(String),

    #[error("Shader compilation failed: {0}")]
    ShaderCompilationFailed(String),

    #[error("Pipeline creation failed: {0}")]
    PipelineCreationFailed(String),

    #[error("Texture error: {0}")]
    TextureError(String),

    #[error("Buffer error: {0}")]
    BufferError(String),

    #[error("Surface error: {0}")]
    SurfaceError(String),

    #[error("Render pass error: {0}")]
    RenderPassError(String),
}
`

- [ ] **Step 2: Create engine-core error type**

Create crates/engine-core/src/error.rs:

`ust
use thiserror::Error;

/// Top-level engine error that aggregates all subsystem errors.
#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Math error: {0}")]
    Math(#[from] engine_math::MathError),

    #[error("ECS error: {0}")]
    Ecs(#[from] engine_ecs::EcsError),

    #[error("Window error: {0}")]
    Window(#[from] engine_window::WindowError),

    #[error("Input error: {0}")]
    Input(#[from] engine_input::InputError),

    #[error("Asset error: {0}")]
    Asset(#[from] engine_asset::AssetError),

    #[error("Scene error: {0}")]
    Scene(#[from] engine_scene::SceneError),

    #[error("Render error: {0}")]
    Render(#[from] engine_render::RenderError),

    #[error("Audio error: {0}")]
    Audio(#[from] engine_audio::AudioError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Initialization failed: {0}")]
    InitFailed(String),
}
`

- [ ] **Step 3: Create engine-framework error type**

Create crates/engine-framework/src/error.rs:

`ust
use thiserror::Error;

/// Errors that can occur in the framework module.
#[derive(Error, Debug)]
pub enum FrameworkError {
    #[error("State not found: {0}")]
    StateNotFound(String),

    #[error("State stack empty")]
    StackEmpty,

    #[error("Invalid state transition: {from} -> {to}")]
    InvalidTransition { from: String, to: String },

    #[error("Save/load error: {0}")]
    SaveLoadError(String),
}
`

- [ ] **Step 4: Create engine-physics error type**

Create crates/engine-physics/src/error.rs:

`ust
use thiserror::Error;

/// Errors that can occur in the physics module.
#[derive(Error, Debug)]
pub enum PhysicsError {
    #[error("Invalid rigid body: {0}")]
    InvalidRigidBody(String),

    #[error("Invalid collider: {0}")]
    InvalidCollider(String),

    #[error("Collision detection error: {0}")]
    CollisionError(String),

    #[error("Solver convergence failed after {0} iterations")]
    SolverConvergence(u32),

    #[error("Invalid joint: {0}")]
    InvalidJoint(String),
}
`

- [ ] **Step 5: Create engine-network error type**

Create crates/engine-network/src/error.rs:

`ust
use thiserror::Error;

/// Errors that can occur in the network module.
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection timeout")]
    Timeout,

    #[error("Disconnected: {0}")]
    Disconnected(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
`

- [ ] **Step 6: Create engine-script error type**

Create crates/engine-script/src/error.rs:

`ust
use thiserror::Error;

/// Errors that can occur in the script module.
#[derive(Error, Debug)]
pub enum ScriptError {
    #[error("Script compilation failed: {0}")]
    CompilationFailed(String),

    #[error("Script runtime error: {0}")]
    RuntimeError(String),

    #[error("Lua error: {0}")]
    LuaError(String),

    #[error("WASM error: {0}")]
    WasmError(String),

    #[error("Hot reload failed: {0}")]
    HotReloadFailed(String),

    #[error("Sandbox violation: {0}")]
    SandboxViolation(String),
}
`

- [ ] **Step 7: Create engine-ui error type**

Create crates/engine-ui/src/error.rs:

`ust
use thiserror::Error;

/// Errors that can occur in the UI module.
#[derive(Error, Debug)]
pub enum UiError {
    #[error("Widget not found: {0}")]
    WidgetNotFound(String),

    #[error("Layout error: {0}")]
    LayoutError(String),

    #[error("Theme error: {0}")]
    ThemeError(String),

    #[error("Font error: {0}")]
    FontError(String),
}
`

- [ ] **Step 8: Create engine-terrain error type**

Create crates/engine-terrain/src/error.rs:

`ust
use thiserror::Error;

/// Errors that can occur in the terrain module.
#[derive(Error, Debug)]
pub enum TerrainError {
    #[error("Invalid heightmap: {0}")]
    InvalidHeightmap(String),

    #[error("Invalid layer: {0}")]
    InvalidLayer(String),

    #[error("Generation failed: {0}")]
    GenerationFailed(String),

    #[error("Sculpt error: {0}")]
    SculptError(String),
}
`

- [ ] **Step 9: Register all error modules in lib.rs files**

For each of the 8 crates above, add to their lib.rs:

`ust
pub mod error;
pub use error::XxxError;
`

Where XxxError matches the crate's error type name.

- [ ] **Step 10: Verify compilation**

Run: cargo build --all
Expected: All crates compile without errors

- [ ] **Step 11: Commit**

`ash
git add crates/engine-render/src/ crates/engine-core/src/
git add crates/engine-framework/src/ crates/engine-physics/src/
git add crates/engine-network/src/ crates/engine-script/src/
git add crates/engine-ui/src/ crates/engine-terrain/src/
git commit -m "feat: add error types for Layer 3-5 crates (8 crates)"
`

---

### Task 8: Audit engine-math for unwrap()/expect()

**Files:**
- Modify: All .rs files in crates/engine-math/src/

- [ ] **Step 1: Find all unwrap/expect calls**

Run: g "unwrap\(\)|expect\(" crates/engine-math/src/ --glob "!*.md"
List all occurrences with file paths and line numbers.

- [ ] **Step 2: Replace with proper error handling**

For each occurrence:
- If in a function returning Result, use ? operator
- If in a function not returning Result, change return type to Result<T, MathError>
- If the unwrap is truly safe (e.g., on a known-valid value), add a comment explaining why

Example transformation:
`ust
// Before
fn normalize(&self) -> Vec3 {
    let len = self.length();
    *self / len  // panics if len == 0
}

// After
fn normalize(&self) -> Result<Vec3, MathError> {
    let len = self.length();
    if len == 0.0 {
        return Err(MathError::DivisionByZero);
    }
    Ok(*self / len)
}
`

- [ ] **Step 3: Verify tests still pass**

Run: cargo test -p engine-math
Expected: All tests PASS

- [ ] **Step 4: Commit**

`ash
git add crates/engine-math/src/
git commit -m "refactor(math): replace unwrap/expect with proper error handling"
`

---

### Task 9: Audit engine-ecs for unwrap()/expect()

**Files:**
- Modify: All .rs files in crates/engine-ecs/src/

- [ ] **Step 1: Find all unwrap/expect calls**

Run: g "unwrap\(\)|expect\(" crates/engine-ecs/src/ --glob "!*.md"

- [ ] **Step 2: Replace with proper error handling**

Same pattern as Task 8. Key areas:
- Entity lookup failures -> EcsError::EntityNotFound
- Component access failures -> EcsError::ComponentNotRegistered
- World lock violations -> EcsError::WorldLocked

- [ ] **Step 3: Verify tests still pass**

Run: cargo test -p engine-ecs
Expected: All tests PASS

- [ ] **Step 4: Commit**

`ash
git add crates/engine-ecs/src/
git commit -m "refactor(ecs): replace unwrap/expect with proper error handling"
`

---

### Task 10: Audit remaining crates for unwrap()/expect()

**Files:**
- Modify: All .rs files in remaining crates

- [ ] **Step 1: Audit engine-window**

Run: g "unwrap\(\)|expect\(" crates/engine-window/src/
Replace with WindowError where appropriate.

- [ ] **Step 2: Audit engine-audio**

Run: g "unwrap\(\)|expect\(" crates/engine-audio/src/
Replace with AudioError where appropriate.

- [ ] **Step 3: Audit engine-asset**

Run: g "unwrap\(\)|expect\(" crates/engine-asset/src/
Replace with AssetError where appropriate.

- [ ] **Step 4: Audit engine-scene**

Run: g "unwrap\(\)|expect\(" crates/engine-scene/src/
Replace with SceneError where appropriate.

- [ ] **Step 5: Audit engine-input**

Run: g "unwrap\(\)|expect\(" crates/engine-input/src/
Replace with InputError where appropriate.

- [ ] **Step 6: Audit engine-render**

Run: g "unwrap\(\)|expect\(" crates/engine-render/src/
Replace with RenderError where appropriate.

- [ ] **Step 7: Audit engine-core**

Run: g "unwrap\(\)|expect\(" crates/engine-core/src/
Replace with EngineError where appropriate.

- [ ] **Step 8: Audit engine-framework**

Run: g "unwrap\(\)|expect\(" crates/engine-framework/src/
Replace with FrameworkError where appropriate.

- [ ] **Step 9: Audit engine-physics**

Run: g "unwrap\(\)|expect\(" crates/engine-physics/src/
Replace with PhysicsError where appropriate.

- [ ] **Step 10: Audit engine-network**

Run: g "unwrap\(\)|expect\(" crates/engine-network/src/
Replace with NetworkError where appropriate.

- [ ] **Step 11: Audit engine-script**

Run: g "unwrap\(\)|expect\(" crates/engine-script/src/
Replace with ScriptError where appropriate.

- [ ] **Step 12: Audit engine-ui**

Run: g "unwrap\(\)|expect\(" crates/engine-ui/src/
Replace with UiError where appropriate.

- [ ] **Step 13: Audit engine-terrain**

Run: g "unwrap\(\)|expect\(" crates/engine-terrain/src/
Replace with TerrainError where appropriate.

- [ ] **Step 14: Audit engine-editor**

Run: g "unwrap\(\)|expect\(" crates/engine-editor/src/
Replace with appropriate error types.

- [ ] **Step 15: Verify all tests pass**

Run: cargo test --all
Expected: All tests PASS

- [ ] **Step 16: Commit**

`ash
git add crates/
git commit -m "refactor: replace unwrap/expect with proper error handling across all crates"
`

---

### Task 11: Validate module dependency rules

**Files:**
- Create: docs/architecture.md (update existing)

- [ ] **Step 1: Verify no circular dependencies**

Run: cargo tree --workspace --depth 1
Check that no crate depends on itself (directly or transitively).

- [ ] **Step 2: Verify no upward dependencies**

Check that:
- Layer 0 crates (math, jobs, window) don't depend on higher layers
- Layer 1 crates (audio, asset, ecs) don't depend on Layer 2+
- Layer 2 crates (scene, input) don't depend on Layer 3+
- etc.

- [ ] **Step 3: Document dependency diagram**

Update docs/architecture.md with the validated layer structure:

`
Layer 0 (Leaf):           engine-math, engine-jobs, engine-window
Layer 1 (Foundation):     engine-audio, engine-asset, engine-ecs
Layer 2 (Infrastructure): engine-scene, engine-input
Layer 3 (Rendering):      engine-render
Layer 4 (Core):           engine-core
Layer 5 (Systems):        engine-framework, engine-physics, engine-network,
                          engine-script, engine-ui, engine-terrain
Layer 6 (Application):    engine-editor
`

- [ ] **Step 4: Commit**

`ash
git add docs/architecture.md
git commit -m "docs: update architecture with validated dependency layers"
`

---

### Task 12: Run final verification

**Files:**
- None (verification only)

- [ ] **Step 1: Run full CI suite**

Run: just ci (or cargo fmt --check && cargo clippy --all && cargo build --all && cargo test --all)
Expected: All checks PASS

- [ ] **Step 2: Verify success criteria**

- [ ] All tests pass: cargo test --all -- 0 failures
- [ ] Error handling unified: g "unwrap\(\)|expect\(" crates/ --glob "!*/tests/*" --glob "!*/examples/*" --glob "!*/benches/*" -- should return 0 matches in production code
- [ ] Module boundaries clean: No circular or upward dependencies
