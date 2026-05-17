//! Debug and logging utilities.
//!
//! Provides a unified interface for logging and crash diagnostics.
//!
//! # Usage
//!
//! ```ignore
//! use engine_core::debug::init_logger;
//! init_logger();
//!
//! // Or via the plugin system:
//! app.add_plugin(engine_core::debug::DebugPlugin);
//! ```
//!
//! Control log level via `RUST_LOG` env var (default: `info`):
//! ```text
//! RUST_LOG=debug ./my_app
//! RUST_LOG=wgpu=warn ./my_app
//! ```

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize logging and panic hook.
///
/// Called once; safe to call multiple times.
///
/// - Allocates a console window on Windows so stdout/stderr are visible.
/// - Sets a panic hook that prints a full backtrace and shows a
///   `MessageBox` on Windows release builds.
/// - Initializes [`env_logger`] with default filter `info`.
pub fn init_logger() {
    INIT.call_once(|| {
        if cfg!(windows) {
            alloc_console();
        }
        set_panic_hook();
        init_env_logger();
        log::info!("logger initialized");
    });
}

pub struct DebugPlugin;

impl crate::plugin::Plugin for DebugPlugin {
    fn build(&self, _app: &mut crate::app::AppBuilder) {
        init_logger();
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn alloc_console() {
    unsafe extern "system" {
        fn AllocConsole() -> i32;
    }
    unsafe {
        AllocConsole();
    }
}

fn set_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let bt = std::backtrace::Backtrace::force_capture();
        eprintln!("===== PANIC =====");
        eprintln!("{info}");
        eprintln!();
        eprintln!("{bt}");
        eprintln!("=================");

        #[cfg(windows)]
        show_error_dialog(&info.to_string());
    }));
}

#[cfg(windows)]
fn show_error_dialog(msg: &str) {
    unsafe extern "system" {
        fn MessageBoxA(
            hWnd: isize,
            lpText: *const std::ffi::c_char,
            lpCaption: *const std::ffi::c_char,
            uType: u32,
        ) -> i32;
    }
    let caption = std::ffi::CString::new("Engine Crash").unwrap();
    let body = std::ffi::CString::new(format!("{msg}\n\nSee console for full backtrace."))
        .unwrap_or_default();
    unsafe {
        MessageBoxA(0, body.as_ptr(), caption.as_ptr(), 0x00000010);
    }
}

fn init_env_logger() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();
}
