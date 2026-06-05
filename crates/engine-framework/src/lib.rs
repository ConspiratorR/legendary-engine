//! High-level game framework built on top of the ECS.
//!
//! Provides a [`GameState`] trait with a [`StateStack`] for managing
//! transitions between title, menu, pause, and game-over screens.
//! The [`GameFlowPlugin`] wires the standard
//! state machine into an [`AppBuilder`](engine_core::app::AppBuilder).
//!
//! # Quick Start
//!
//! ```rust
//! use engine_framework::{StateStack, GameState, StateCtx, FrameworkPlugin, GameFlowPlugin};
//! use engine_core::app::AppBuilder;
//!
//! // 1. Build an app and register the framework + game-flow plugins.
//! let mut builder = AppBuilder::new();
//! builder.add_plugin(FrameworkPlugin);
//! builder.add_plugin(GameFlowPlugin);
//! let mut app = builder.build();
//!
//! // 2. The GameFlowPlugin pushes a TitleState automatically.
//! app.run(); // flushes pending ops → TitleState enters
//!
//! // 3. Drive transitions by inserting GameStateAction resources:
//! //    app.resources_mut().insert(GameStateAction::PushMenu);
//! //    app.run();
//! ```
//!
//! # Architecture
//!
//! | Component | Purpose |
//! |---|---|
//! | [`StateStack`] | Deferred push / pop / replace with lifecycle hooks |
//! | [`GameState`] | Trait for discrete states (title, menu, pause, …) |
//! | [`GameFlowPlugin`] | Wires the standard state machine into the engine |
//! | [`FrameworkPlugin`] | Registers [`StateStack`] and hooks updates into the engine loop |
//! | [`SaveManager`](save::SaveManager) | JSON-based save / load with slot management |
//!
//! # State Lifecycle
//!
//! States on the [`StateStack`] receive lifecycle callbacks during [`StateStack::flush`]:
//!
//! | Transition | Outgoing hook | Incoming hook |
//! |---|---|---|
//! | **Push** new state on top | old top → [`on_pause`](GameState::on_pause) | new state → [`on_enter`](GameState::on_enter) |
//! | **Pop** top state | popped → [`on_exit`](GameState::on_exit) | new top → [`on_resume`](GameState::on_resume) |
//! | **Replace** top state | old top → [`on_exit`](GameState::on_exit) | new state → [`on_enter`](GameState::on_enter) |
//!
//! Only the topmost state receives [`update`](GameState::update) calls each frame.

pub mod error;
pub use error::FrameworkError;

pub mod action;
pub mod ctx;
pub mod flow;
pub mod plugin;
pub mod resource;
pub mod save;
pub mod stack;
pub mod state;
pub mod states;

pub use action::{GameSession, GameStateAction};
pub use ctx::StateCtx;
pub use flow::GameFlowPlugin;
pub use plugin::FrameworkPlugin;
pub use resource::FrameworkResource;
pub use stack::StateStack;
pub use state::GameState;
pub use states::{GameOverState, MenuState, PauseState, TitleState};
