//! Input handling for keyboard and mouse.
//!
//! Provides [`input_manager::InputManager`] for raw key/mouse state tracking and
//! [`ActionMap`](action::ActionMap) for abstracting physical inputs
//! into named game actions.
//!
//! # Example
//!
//! ```
//! use engine_input::input_manager::InputManager;
//! use engine_input::keyboard::KeyCode;
//!
//! let mut input = InputManager::new();
//! input.press(KeyCode::Space);
//! assert!(input.key_down(KeyCode::Space));
//! input.update_frame();
//! assert!(input.key_down(KeyCode::Space)); // still held
//! ```

pub mod action;
pub mod input_manager;
pub mod keyboard;
pub mod mouse;
