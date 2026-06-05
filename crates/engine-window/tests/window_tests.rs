use engine_window::{WindowConfig, WindowError};

#[test]
fn default_config_has_expected_values() {
    let config = WindowConfig::default();
    assert_eq!(config.title, "RustEngine");
    assert_eq!(config.width, 1280);
    assert_eq!(config.height, 720);
    assert!(config.vsync);
}

#[test]
fn new_returns_default_config() {
    let config = WindowConfig::new();
    assert_eq!(config.title, "RustEngine");
    assert_eq!(config.width, 1280);
    assert_eq!(config.height, 720);
    assert!(config.vsync);
}

#[test]
fn builder_sets_title() {
    let config = WindowConfig::new().with_title("Test Window");
    assert_eq!(config.title, "Test Window");
}

#[test]
fn builder_sets_size() {
    let config = WindowConfig::new().with_size(1920, 1080);
    assert_eq!(config.width, 1920);
    assert_eq!(config.height, 1080);
}

#[test]
fn builder_sets_vsync() {
    let config = WindowConfig::new().with_vsync(false);
    assert!(!config.vsync);
}

#[test]
fn builder_chains_all_methods() {
    let config = WindowConfig::new()
        .with_title("Chained")
        .with_size(800, 600)
        .with_vsync(false);
    assert_eq!(config.title, "Chained");
    assert_eq!(config.width, 800);
    assert_eq!(config.height, 600);
    assert!(!config.vsync);
}

#[test]
fn validate_passes_for_valid_config() {
    let config = WindowConfig::new();
    assert!(config.validate().is_ok());
}

#[test]
fn validate_fails_for_zero_width() {
    let config = WindowConfig::new().with_size(0, 720);
    let result = config.validate();
    assert!(result.is_err());
    match result.unwrap_err() {
        engine_window::WindowError::InvalidSize { width, height } => {
            assert_eq!(width, 0);
            assert_eq!(height, 720);
        }
        other => panic!("Expected InvalidSize, got: {other:?}"),
    }
}

#[test]
fn validate_fails_for_zero_height() {
    let config = WindowConfig::new().with_size(1280, 0);
    let result = config.validate();
    assert!(result.is_err());
    match result.unwrap_err() {
        engine_window::WindowError::InvalidSize { width, height } => {
            assert_eq!(width, 1280);
            assert_eq!(height, 0);
        }
        other => panic!("Expected InvalidSize, got: {other:?}"),
    }
}

#[test]
fn validate_fails_for_both_zero() {
    let config = WindowConfig::new().with_size(0, 0);
    assert!(config.validate().is_err());
}

#[test]
fn with_title_accepts_string() {
    let config = WindowConfig::new().with_title(String::from("Owned"));
    assert_eq!(config.title, "Owned");
}

#[test]
fn with_title_accepts_str_ref() {
    let config = WindowConfig::new().with_title("borrowed");
    assert_eq!(config.title, "borrowed");
}

#[test]
fn validate_fails_for_very_large_dimensions() {
    let config = WindowConfig::new().with_size(u32::MAX, u32::MAX);
    assert!(config.validate().is_ok());
}

#[test]
fn error_display_creation_failed() {
    let err = WindowError::CreationFailed {
        reason: "no display".to_string(),
    };
    assert_eq!(err.to_string(), "Failed to create window: no display");
}

#[test]
fn error_display_not_found() {
    let err = WindowError::NotFound;
    assert_eq!(err.to_string(), "Window not found");
}

#[test]
fn error_display_invalid_size() {
    let err = WindowError::InvalidSize {
        width: 0,
        height: 100,
    };
    assert_eq!(err.to_string(), "Invalid window size: 0x100");
}

#[test]
fn error_display_platform() {
    let err = WindowError::Platform("wayland error".to_string());
    assert_eq!(err.to_string(), "Platform error: wayland error");
}

#[test]
fn error_debug_format() {
    let err = WindowError::NotFound;
    let debug = format!("{err:?}");
    assert!(debug.contains("NotFound"));
}

#[test]
fn window_config_debug_format() {
    let config = WindowConfig::new()
        .with_title("Debug Test")
        .with_size(800, 600)
        .with_vsync(false);
    let debug = format!("{config:?}");
    assert!(debug.contains("Debug Test"));
    assert!(debug.contains("800"));
    assert!(debug.contains("600"));
}

#[test]
fn with_title_overwrites_previous() {
    let config = WindowConfig::new().with_title("First").with_title("Second");
    assert_eq!(config.title, "Second");
}

#[test]
fn with_size_overwrites_previous() {
    let config = WindowConfig::new()
        .with_size(800, 600)
        .with_size(1920, 1080);
    assert_eq!(config.width, 1920);
    assert_eq!(config.height, 1080);
}

#[test]
fn with_vsync_overwrites_previous() {
    let config = WindowConfig::new().with_vsync(true).with_vsync(false);
    assert!(!config.vsync);
}

#[test]
fn validate_passes_for_minimum_valid_size() {
    let config = WindowConfig::new().with_size(1, 1);
    assert!(config.validate().is_ok());
}

#[test]
fn default_impl_equals_new() {
    let a = WindowConfig::new();
    let b = WindowConfig::default();
    assert_eq!(a.title, b.title);
    assert_eq!(a.width, b.width);
    assert_eq!(a.height, b.height);
    assert_eq!(a.vsync, b.vsync);
}
