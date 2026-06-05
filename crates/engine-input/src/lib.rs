//! # engine-input
//!
//! Input management for the RustEngine.
//!
//! Provides keyboard/mouse state tracking, action mapping,
//! and input action detection for game controls.
//!
//! ## Quick Start
//!
//! ```rust
//! use engine_input::input_manager::InputManager;
//! use engine_input::action::ActionMap;
//! use engine_input::keyboard::KeyCode;
//!
//! let mut input = InputManager::new();
//! let mut actions = ActionMap::new();
//!
//! // Bind a key to an action
//! actions.bind_key("jump", KeyCode::Space);
//!
//! // Simulate a key press
//! input.press(KeyCode::Space);
//! actions.update(&input);
//!
//! // Check if action was just pressed
//! if actions.action("jump").just_pressed() {
//!     // Player jumps!
//! }
//! ```

pub mod action;
pub mod error;
pub mod input_manager;
pub mod keyboard;
pub mod mouse;

pub use error::InputError;
