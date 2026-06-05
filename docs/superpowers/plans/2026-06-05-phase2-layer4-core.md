# Phase 2: Layer 4 Core Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** Polish engine-core (the God crate) with tests, docs, and coupling audit. This is the most critical crate -- it depends on 7 other engine crates and 8 crates depend on it.

**Architecture:** Focus on testing the plugin system, app builder, and time management. Audit coupling to identify dependencies that could be optional or removed.

**Tech Stack:** Rust 2024 edition, thiserror, anyhow, cargo test, cargo clippy

---

### Task 1: Polish engine-core

**Files:**
- Modify: crates/engine-core/src/*.rs
- Create: crates/engine-core/tests/core_tests.rs

- [ ] **Step 1: Add module-level documentation**

Add to crates/engine-core/src/lib.rs:

`ust
//! # engine-core
//!
//! Core engine systems for the RustEngine.
//!
//! This is the central crate that ties together all engine subsystems:
//! - Application builder and plugin system
//! - Time management
//! - Configuration system
//! - Logging and performance profiling
//!
//! ## Architecture
//!
//! engine-core follows a plugin-based architecture:
//!
//! `	ext
//! AppBuilder -> Plugin::build() -> System Registration -> App::run()
//! `
//!
//! Each subsystem (render, physics, audio, etc.) is integrated
//! as a plugin that registers its systems with the app builder.
//!
//! ## Quick Start
//!
//! `ust
//! use engine_core::app::{App, AppBuilder};
//! use engine_core::plugin::Plugin;
//!
//! struct MyPlugin;
//! impl Plugin for MyPlugin {
//!     fn build(&self, app: &mut AppBuilder) {
//!         app.add_system(my_system);
//!     }
//! }
//!
//! let mut app = App::new();
//! app.add_plugin(MyPlugin);
//! app.run();
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

Document all public types and functions with /// docs. Focus on:
- AppBuilder API
- Plugin trait
- Time resource
- Configuration API

- [ ] **Step 3: Add plugin system tests**

Create crates/engine-core/tests/core_tests.rs:

`ust
use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;
use engine_core::time::Time;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

#[test]
fn test_app_creation() {
    let app = App::new();
    assert!(app.is_ok());
}

#[test]
fn test_plugin_registration() {
    let mut app = App::new().unwrap();

    struct TestPlugin;
    impl Plugin for TestPlugin {
        fn build(&self, app: &mut AppBuilder) {
            app.add_resource(TestResource { value: 42 });
        }
    }

    struct TestResource {
        value: i32,
    }

    app.add_plugin(TestPlugin);
    let resource = app.resource::<TestResource>();
    assert_eq!(resource.value, 42);
}

#[test]
fn test_multiple_plugins() {
    let mut app = App::new().unwrap();

    struct PluginA;
    impl Plugin for PluginA {
        fn build(&self, app: &mut AppBuilder) {
            app.add_resource(ResourceA { value: 1 });
        }
    }

    struct PluginB;
    impl Plugin for PluginB {
        fn build(&self, app: &mut AppBuilder) {
            app.add_resource(ResourceB { value: 2 });
        }
    }

    struct ResourceA { value: i32 }
    struct ResourceB { value: i32 }

    app.add_plugin(PluginA);
    app.add_plugin(PluginB);

    assert_eq!(app.resource::<ResourceA>().value, 1);
    assert_eq!(app.resource::<ResourceB>().value, 2);
}

#[test]
fn test_system_execution() {
    let mut app = App::new().unwrap();
    let counter = Arc::new(AtomicI32::new(0));
    let counter_clone = counter.clone();

    struct CounterPlugin(Arc<AtomicI32>);
    impl Plugin for CounterPlugin {
        fn build(&self, app: &mut AppBuilder) {
            let counter = self.0.clone();
            app.add_system(move |_app: &App| {
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }
    }

    app.add_plugin(CounterPlugin(counter_clone));
    app.update();

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_system_ordering() {
    let mut app = App::new().unwrap();
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));

    struct OrderPlugin(Arc<std::sync::Mutex<Vec<i32>>>);
    impl Plugin for OrderPlugin {
        fn build(&self, app: &mut AppBuilder) {
            let order = self.0.clone();
            app.add_system(move |_app: &App| {
                order.lock().unwrap().push(1);
            });
            let order = self.0.clone();
            app.add_system(move |_app: &App| {
                order.lock().unwrap().push(2);
            });
        }
    }

    app.add_plugin(OrderPlugin(order.clone()));
    app.update();

    let order = order.lock().unwrap();
    assert_eq!(*order, vec![1, 2]);
}
`

- [ ] **Step 4: Add time management tests**

Add to crates/engine-core/tests/core_tests.rs:

`ust
#[test]
fn test_time_resource() {
    let time = Time::new();
    assert_eq!(time.delta_seconds(), 0.0);
    assert_eq!(time.elapsed_seconds(), 0.0);
    assert_eq!(time.fps(), 0.0);
}

#[test]
fn test_time_update() {
    let mut time = Time::new();
    time.update(std::time::Duration::from_millis(16));

    assert!((time.delta_seconds() - 0.016).abs() < 0.001);
    assert!((time.elapsed_seconds() - 0.016).abs() < 0.001);
    assert!((time.fps() - 62.5).abs() < 1.0);
}

#[test]
fn test_time_multiple_updates() {
    let mut time = Time::new();
    for _ in 0..60 {
        time.update(std::time::Duration::from_millis(16));
    }

    assert!((time.elapsed_seconds() - 0.96).abs() < 0.01);
}
`

- [ ] **Step 5: Add configuration tests**

Add to crates/engine-core/tests/core_tests.rs:

`ust
use engine_core::config::Config;

#[test]
fn test_config_default() {
    let config = Config::default();
    assert!(config.is_ok());
}

#[test]
fn test_config_get_set() {
    let mut config = Config::default().unwrap();
    config.set("volume", 0.8);
    assert_eq!(config.get::<f32>("volume"), Some(0.8));
}

#[test]
fn test_config_missing_key() {
    let config = Config::default().unwrap();
    assert_eq!(config.get::<f32>("nonexistent"), None);
}
`

- [ ] **Step 6: Run tests**

Run: cargo test -p engine-core
Expected: All tests PASS

- [ ] **Step 7: Coupling audit**

Analyze crates/engine-core/Cargo.toml dependencies:

`	oml
[dependencies]
engine-ecs = { path = "../engine-ecs" }
engine-scene = { path = "../engine-scene" }
engine-render = { path = "../engine-render" }
engine-input = { path = "../engine-input" }
engine-asset = { path = "../engine-asset" }
engine-window = { path = "../engine-window" }
engine-math = { path = "../engine-math" }
engine-audio = { path = "../engine-audio", optional = true }
`

For each dependency, document:
1. Why it's needed
2. Whether it could be optional
3. Whether the dependency could be inverted (dependency injection)

Create docs/core-coupling-audit.md with findings.

- [ ] **Step 8: Run clippy**

Run: cargo clippy -p engine-core
Expected: Zero warnings

- [ ] **Step 9: Commit**

`ash
git add crates/engine-core/
git add docs/core-coupling-audit.md
git commit -m "feat(core): add docs, tests, coupling audit for engine-core"
`

---

### Task 2: Final verification for Layer 4

**Files:**
- None (verification only)

- [ ] **Step 1: Run full test suite**

Run: cargo test -p engine-core
Expected: All tests PASS

- [ ] **Step 2: Run clippy**

Run: cargo clippy -p engine-core
Expected: Zero warnings

- [ ] **Step 3: Verify documentation**

Run: cargo doc -p engine-core --no-deps
Expected: No warnings about missing docs
