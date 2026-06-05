//! High-level game framework built on top of the ECS.
//!
//! Provides a [`GameState`] trait with a [`StateStack`] for managing
//! transitions between title, menu, pause, and game-over screens.
//! The [`GameFlowPlugin`] wires the standard
//! state machine into an [`AppBuilder`](engine_core::app::AppBuilder).

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
