# Next Iteration: Quality, Architecture & Documentation

**Date:** 2026-06-05
**Status:** Approved
**Scope:** Full-stack quality improvement, architecture refactoring, documentation & DX
**Strategy:** Bottom-up gradual approach (hybrid)

---

## Overview

RustEngine has completed all 9 development stages (17 crates, ~9k+ lines). This iteration focuses on hardening the existing codebase rather than adding new features. Three pillars:

1. **Quality & Stability** -- fix test failures, increase coverage, optimize performance, audit code quality
2. **Architecture Refactoring** -- clean module boundaries, unify APIs, extract abstractions, standardize error handling
3. **Documentation & DX** -- API docs, examples, architecture docs, workflow optimization

---

## Phase 1: Cross-Cutting Foundations

### 1.1 Fix Known Test Failures

| Crate | Issue | Fix |
|-------|-------|-----|
| engine-asset | Missing tempfile dev-dependency | Add to Cargo.toml |
| engine-core | Outdated KeyCode variants in examples | Update to current winit key codes |

**Verification:** cargo test --all passes cleanly.

### 1.2 Error Handling Unification

**Goal:** Consistent error types across all crates, proper error propagation.

**Design:**

`
engine-core/src/error.rs -> EngineError (top-level enum)
    |
Each crate defines XxxError via thiserror:
    engine-render/src/error.rs  -> RenderError
    engine-physics/src/error.rs -> PhysicsError
    engine-asset/src/error.rs   -> AssetError
    engine-audio/src/error.rs   -> AudioError
    engine-network/src/error.rs -> NetworkError
    engine-scene/src/error.rs   -> SceneError
    engine-input/src/error.rs   -> InputError
    engine-script/src/error.rs  -> ScriptError
    engine-terrain/src/error.rs -> TerrainError
    engine-jobs/src/error.rs    -> JobsError
`

**Rules:**
- Public functions return anyhow::Result<T> or Result<T, XxxError>
- Internal functions use ? operator, never unwrap()/expect() in production code
- unwrap()/expect() allowed only in tests and benchmarks
- Each XxxError implements std::error::Error + Send + Sync
- EngineError has variants for each subsystem error

**Audit checklist per crate:**
- [ ] Grep for unwrap() and expect() -- replace with ? or proper error handling
- [ ] Ensure all public functions have documented error conditions
- [ ] Verify error messages are descriptive and actionable

### 1.3 Module Boundary Analysis & Dependency Rules

**Validated layer structure (from Cargo.toml analysis):**

`
Layer 0 (Leaf):           engine-math, engine-jobs, engine-window
Layer 1 (Foundation):     engine-audio, engine-asset, engine-ecs
Layer 2 (Infrastructure): engine-scene, engine-input
Layer 3 (Rendering):      engine-render
Layer 4 (Core):           engine-core (God crate -- depends on 7 engine crates)
Layer 5 (Systems):        engine-framework, engine-physics, engine-network,
                          engine-script, engine-ui, engine-terrain
Layer 6 (Application):    engine-editor (depends on 11 engine crates)
`

**Key findings:**
- engine-core is a "God crate" with 7 mandatory deps and 8 reverse deps
- engine-physics, engine-network, engine-script are isolated (no reverse deps)
- engine-jobs is effectively orphaned (only optional dep of engine-ecs)
- No circular dependencies exist -- the graph is a valid DAG

**Rules:**
- Dependencies only flow downward (Layer N -> Layer N-1 or lower)
- No circular dependencies between crates
- No upward dependencies
- Cross-layer references within the same level are allowed

**Deliverable:** Dependency diagram in docs/architecture.md with violation report.

---

## Phase 2: Bottom-Up Per-Crate Polish

### Execution Order

Process each crate bottom-up. For each crate, complete ALL items before moving to the next:

| # | Crate | Layer | Extra Focus |
|---|-------|-------|-------------|
| 1 | engine-math | 0 | API docs, edge-case tests, benchmarks |
| 2 | engine-jobs | 0 | Concurrency tests, scheduler docs |
| 3 | engine-window | 0 | Error handling, platform compat tests |
| 4 | engine-audio | 1 | Playback tests, mixer docs |
| 5 | engine-asset | 1 | Test fix, loading pipeline docs |
| 6 | engine-ecs | 1 | Query iteration perf, memory layout audit |
| 7 | engine-scene | 2 | Hierarchy sync tests, serialization docs |
| 8 | engine-input | 2 | API consistency, action mapping docs |
| 9 | engine-render | 3 | Deferred rendering tests, GPU resource lifecycle |
| 10 | engine-core | 4 | Test fix, plugin system docs, coupling audit |
| 11 | engine-framework | 5 | State stack tests, lifecycle docs |
| 12 | engine-physics | 5 | Collision detection tests, broadphase perf |
| 13 | engine-network | 5 | Connection tests, protocol docs |
| 14 | engine-script | 5 | WASM/Lua integration tests, sandbox safety |
| 15 | engine-ui | 5 | Component API docs, examples |
| 16 | engine-terrain | 5 | Terrain generation tests, editor integration docs |
| 17 | engine-editor | 6 | UI component tests, workflow docs |

### Per-Crate Checklist (Universal)

For each crate:

- [ ] **Error migration:** Create error.rs with XxxError using thiserror
- [ ] **Eliminate unwrap/expect:** Grep and replace with proper error handling
- [ ] **Unit tests:** Target >80% coverage on core logic
- [ ] **Module docs:** Add crate-level and module-level documentation
- [ ] **Function docs:** Add /// to all public functions with examples
- [ ] **API consistency:** Check naming conventions (verb/noun style)
- [ ] **Clippy clean:** cargo clippy -p <crate> with zero warnings
- [ ] **Benchmarks:** Add Criterion benchmarks for performance-critical paths

### Per-Crate Extra Focus

- **engine-ecs:** Query iteration hot paths, archetype storage efficiency, memory allocation patterns
- **engine-render:** GPU resource lifecycle (textures, buffers, pipelines), shader compilation caching, render graph execution overhead
- **engine-physics:** Broadphase sweep-and-prune efficiency, contact solver convergence, CCD accuracy
- **engine-network:** Message serialization/deserialization overhead, connection state machine correctness
- **engine-script:** WASM sandbox isolation, hot-reload reliability, Lua-ECS bridge safety
- **engine-core:** Coupling audit -- identify which deps could be optional or removed

---

## Phase 3: Integration Verification & Documentation

### 3.1 Integration Tests

- Cross-crate integration tests:
  - ECS + Render: spawn entity with Sprite/Mesh -> verify GPU upload
  - ECS + Physics: spawn RigidBody -> verify simulation step
  - Scene + Asset: load scene file -> verify entity hierarchy
  - Script + ECS: Lua script modifies component -> verify change propagates
- Ensure cargo test --all passes on Windows, Linux, macOS
- Fix flaky tests in CI matrix

### 3.2 Example Code

- One minimal example per major subsystem (updated to current API)
- One combined example showing multi-system collaboration
- Update all existing examples to compile with current API
- Ensure cargo run --example <name> works for each

### 3.3 Architecture Documentation

- **docs/architecture.md:** Crate dependency diagram, data flow, design decisions
- **docs/contributing.md:** Code style, error handling conventions, testing conventions
- Update per-crate README.md if present

### 3.4 Development Workflow

- Audit justfile for missing commands
- Ensure just ci (fmt -> clippy -> build -> test) runs cleanly
- Consider adding just check for quick local validation

---

## Success Criteria

| Criteria | Metric |
|----------|--------|
| All tests pass | cargo test --all -- 0 failures |
| Zero clippy warnings | cargo clippy --all -- clean |
| Error handling unified | No unwrap()/expect() outside tests/benches |
| Module boundaries clean | No circular or upward dependencies |
| Test coverage | Core logic >80% per crate |
| Documentation | Every public function has /// docs |
| All examples compile | cargo build --examples -- clean |
| CI passes | GitHub Actions matrix green |

---

## Out of Scope

- New features or subsystems
- Major version bumps
- Breaking API changes (internal refactoring only)
- Editor UI redesign
- Performance optimization beyond identified bottlenecks
