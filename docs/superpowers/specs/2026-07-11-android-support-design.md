# Android Target Support Design

> **Goal:** Enable RustEngine to compile and run on Android devices (aarch64-linux-android), with working rendering, input, audio, and asset loading.

**Architecture:** Add `#[cfg(target_os = "android")]` code paths in existing crates rather than creating new crates. The entry point uses `android-activity` crate's `android_main` macro. Assets load from APK via `ndk::asset::AssetManager`. Touch input comes from winit's `WindowEvent::Touch`. Audio uses rodio's oboe backend.

**Tech Stack:** android-activity 0.6, ndk 0.9, ndk-context 0.1, wgpu (Vulkan backend), rodio (oboe)

---

## Scope

### In Scope
- Android entry point (`android_main`)
- Window creation deferred to activity resume
- wgpu rendering on Android (Vulkan)
- Touch input handling
- Asset loading from APK assets directory
- Android logcat logging
- Minimal runnable example

### Out of Scope
- iOS support (separate effort)
- Google Play Store publishing
- Android-specific UI (soft keyboard, system UI)
- Dynamic library plugins on Android
- CI/CD for Android builds

---

## Components

### 1. Entry Point (`engine-core`)

Android has no `main()`. The entry point is `android_main(android_app: AndroidApp)` provided by `android-activity`. We need:

- A new binary target or `[[bin]]` section that compiles only on Android
- The `android_main` function that creates an `EventLoop` with the `AndroidApp` handle
- The event loop handles `Resumed` (create window + renderer), `Suspended` (pause), `SaveState`, and standard window events

The existing `run_default()` function stays unchanged for desktop. A new `run_android()` function handles the Android lifecycle.

### 2. Window Creation (`engine-window`)

On Android, `EventLoop::new()` takes an `&AndroidApp` parameter. Window creation must be deferred until `Event::Resumed` because the native window isn't available until then.

Changes:
- Add `create_event_loop_android(app: &AndroidApp)` function
- Modify `create_window()` to work with the Android event loop's `WindowAttributes`

### 3. Asset Loading (`engine-asset`)

Android assets live inside the APK and are accessed via `ndk::asset::AssetManager`. The current filesystem-based loading won't work.

Changes:
- Gate `notify` file watcher behind `#[cfg(not(target_os = "android"))]`
- Add `AndroidAssetLoader` that reads from `AssetManager`
- Feature flag or cfg-gate to select the right loader at compile time

### 4. Touch Input (`engine-input`)

winit provides `WindowEvent::Touch` on Android. We need to forward these to the input system.

Changes:
- Add `TouchEvent` struct and `TouchPhase` enum
- Handle `WindowEvent::Touch` in the event loop
- Expose touch state through the input resource

### 5. Audio (`engine-audio`)

rodio supports Android via the `oboe` backend. May need feature flags.

Changes:
- Add `oboe` feature to rodio dependency for Android
- Test audio playback on device

### 6. Logging (`engine-core`)

`env_logger` doesn't output to Android logcat. Use `android_logger` crate.

Changes:
- Add `android_logger` dependency (Android-only)
- Initialize with `android_logger::init_once()` on Android
- Gate `env_logger` behind `#[cfg(not(target_os = "android"))]`

### 7. Build Configuration

- `Cargo.toml`: Add `android_logger` to workspace dependencies
- `justfile`: Verify `build-android` recipe works
- Add `android` example binary

---

## Data Flow

```
android_main(android_app)
  -> EventLoop::with_android_app(&android_app)
  -> match event:
       Resumed => create_window() + init_renderer()
       Touch => forward to input system
       AboutToWait => app.run() + render()
       Suspended => pause audio
  -> event_loop.run()
```

---

## Testing

- Cross-compile: `cargo build --target aarch64-linux-android`
- Run on device via `adb` or cargo-ndk
- Verify: window renders, touch input works, audio plays, assets load

---

## Risk Mitigation

- **wgpu on Android:** Vulkan is the standard GPU API; wgpu supports it natively. Low risk.
- **Asset loading:** The ndk AssetManager API is well-documented. Medium complexity.
- **Audio:** rodio's oboe backend is maintained. Low risk.
- **Build toolchain:** cargo-ndk is the standard approach. Low risk.
