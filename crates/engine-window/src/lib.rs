//! # engine-window
//!
//! Window management for the RustEngine.
//!
//! Wraps [`winit`] 0.30 to provide cross-platform window creation
//! and event handling. Supports Windows, macOS, and Linux
//! (Wayland/X11).
//!
//! ## Architecture
//!
//! This crate is Layer 0 of the engine — it owns the OS window and
//! event loop. Higher layers (rendering, input) consume the
//! [`Window`] handle produced by [`create_window`].
//!
//! - [`WindowConfig`] — builder for window parameters (title, size, vsync).
//! - [`create_window`] — creates a [`winit::window::Window`] from a config
//!   and an [`EventLoop`](winit::event_loop::EventLoop).
//! - [`WindowError`] — error type covering creation failures and invalid configs.
//!
//! ## Platform Notes
//!
//! | Platform | Backend | Notes |
//! |----------|---------|-------|
//! | Windows  | Win32   | DPI-aware via per-monitor v2. |
//! | macOS    | AppKit  | Retina scaling handled by winit. |
//! | Linux    | Wayland / X11 | Auto-detected; override with `WINIT_UNIX_BACKEND`. |
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use engine_window::WindowConfig;
//!
//! let config = WindowConfig::new()
//!     .with_title("My Game")
//!     .with_size(1280, 720);
//! ```
//!
//! ## Error Handling
//!
//! All fallible operations return [`Result<T, WindowError>`]. The error
//! variants distinguish between platform-level creation failures
//! ([`WindowError::CreationFailed`]) and configuration validation
//! errors ([`WindowError::InvalidSize`]).

pub mod error;
pub mod window;
pub use error::WindowError;
pub use window::{WindowConfig, create_window};
