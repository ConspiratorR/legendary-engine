//! Minimal Android demo for RustEngine.
//!
//! Build: cargo build --target aarch64-linux-android --example android_demo
//! Run: adb push target/aarch64-linux-android/debug/android_demo /data/local/tmp/
//!      adb shell /data/local/tmp/android_demo

#[cfg(target_os = "android")]
fn main() {
    env_logger::init();
    log::info!("RustEngine Android demo starting");

    let mut app_builder = engine_core::Engine::new();
    // Add plugins as needed
    // app_builder.add_plugin(engine_core::plugins::CorePlugins);

    if let Err(e) = engine_core::android::run_android(app_builder) {
        log::error!("Android demo failed: {e}");
    }
}

#[cfg(not(target_os = "android"))]
fn main() {
    println!("This example is Android-only.");
    println!("Use: cargo run --example basic -p engine-core");
}
