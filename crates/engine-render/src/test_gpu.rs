//! Shared GPU test helpers.
//!
//! Tests using these helpers require a real GPU and will hang in headless environments.
//! Run with: `cargo test -p engine-render -- --ignored`

use pollster::block_on;

/// Create a wgpu Device and Queue with default features and limits.
///
/// # Panics
/// Panics if no GPU adapter is available.
pub fn create_test_device() -> (wgpu::Device, wgpu::Queue) {
    create_test_device_with_features(wgpu::Features::empty(), wgpu::Limits::default())
}

/// Create a wgpu Device and Queue with specific features and limits.
///
/// # Panics
/// Panics if no GPU adapter is available.
pub fn create_test_device_with_features(
    features: wgpu::Features,
    limits: wgpu::Limits,
) -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("Failed to find a GPU adapter");

    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("test device"),
            required_features: features,
            required_limits: limits,
            ..Default::default()
        },
        None,
    ))
    .expect("Failed to create GPU device");

    (device, queue)
}
