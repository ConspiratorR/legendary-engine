use engine_window::WindowConfig;

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
