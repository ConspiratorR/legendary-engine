# Android Target Setup

This guide explains how to set up the Android build environment for RustEngine.

## Phase 30 — 本机基线探测（2026-07-13）

| 项 | 状态 |
|----|------|
| `aarch64-linux-android` rustup target | ✅ 已安装 |
| `cargo-ndk` | ✅ `C:\Users\17415\.cargo\bin\cargo-ndk.exe` |
| `ANDROID_NDK_HOME` / NDK clang | ❌ **本机未检出**（`aarch64-linux-android-clang++` 未找到） |
| `android-activity` feature | ✅ workspace 已启用 `game-activity`（phase 30） |
| `engine-core` 默认 features（含 audio/oboe） | ❌ 需 NDK C++ 工具链 |
| `engine-core --no-default-features --features unity-world-primary` | ❌ 仍需 android-activity 的 NDK 链接 |

**结论（本机）**：代码侧 feature 已就绪；**完整 Android 编译需安装 NDK** 并设置：

```powershell
$env:ANDROID_NDK_HOME = "C:\Users\<user>\AppData\Local\Android\Sdk\ndk\<version>"
$env:PATH += ";$env:ANDROID_NDK_HOME\toolchains\llvm\prebuilt\windows-x86_64\bin"
# 或
cargo ndk -t arm64-v8a build -p engine-core
```

**无 NDK 时推荐命令（预期失败，用于验证环境）**：

```powershell
cargo build -p engine-core --target aarch64-linux-android --no-default-features --features unity-world-primary
```

**有 NDK 时**：见下方 Installation Steps；default features 含 audio 时还需 oboe 的 NDK clang。

## Prerequisites

1. **Android Studio** or **Android SDK Command Line Tools**
2. **Android NDK** (version 25 or later) — **必需**
3. **Rust Android target** (`aarch64-linux-android`)

## Installation Steps

### 1. Install Android Studio

Download and install Android Studio from https://developer.android.com/studio

### 2. Install Android NDK

#### Option A: Via Android Studio
1. Open Android Studio
2. Go to Settings → Appearance & Behavior → System Settings → Android SDK
3. Click "SDK Tools" tab
4. Check "NDK (Side by side)"
5. Click "Apply" to install

#### Option B: Via Command Line
```bash
# Download Android SDK Command Line Tools
# https://developer.android.com/studio#command-tools

# Install NDK
sdkmanager "ndk;25.2.9519653"
```

### 3. Set Environment Variables

```bash
# Windows (PowerShell)
$env:ANDROID_NDK_HOME = "C:\Users\<username>\AppData\Local\Android\Sdk\ndk\25.2.9519653"
$env:ANDROID_HOME = "C:\Users\<username>\AppData\Local\Android\Sdk"

# Linux/macOS
export ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/25.2.9519653"
export ANDROID_HOME="$HOME/Android/Sdk"
```

### 4. Install Rust Android Target

```bash
rustup target add aarch64-linux-android
```

### 5. Install cargo-ndk

```bash
cargo install cargo-ndk
```

## Building for Android

### Using cargo-ndk

```bash
# Build for Android
cargo ndk -t aarch64-linux-android build --release

# Build APK (requires Android SDK)
cargo ndk -t aarch64-linux-android build --release
```

### Manual Build

```bash
# Set environment variables
export ANDROID_NDK_HOME="/path/to/ndk"
export CC_aarch64_linux_android="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang"
export CXX_aarch64_linux_android="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang++"
export AR_aarch64_linux_android="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang"

# Build
cargo build --target aarch64-linux-android --release
```

## Android-Specific Features

### Touch Input

RustEngine supports touch input on Android via `engine_input::touch`:

```rust
use engine_input::touch::{TouchPoint, TouchPhase};

fn handle_touch(input: &InputManager) {
    for touch in input.touch().points.iter() {
        match touch.phase {
            TouchPhase::Started => println!("Touch started at ({}, {})", touch.x, touch.y),
            TouchPhase::Moved => println!("Touch moved to ({}, {})", touch.x, touch.y),
            TouchPhase::Ended => println!("Touch ended"),
            TouchPhase::Cancelled => println!("Touch cancelled"),
        }
    }
}
```

### Asset Loading

On Android, assets are loaded from the APK's assets directory using `ndk::asset::AssetManager`.
The file watcher is automatically disabled on Android since hot-reload is not supported.

### Audio

Android uses AAudio or OpenSL ES for audio via rodio's oboe backend.
Audio playback works the same as on desktop.

## Troubleshooting

### NDK not found

If you get "Could not find any NDK" error:

```bash
# Set ANDROID_NDK_HOME explicitly
export ANDROID_NDK_HOME="/path/to/ndk"

# Or use cargo-ndk-env to check
cargo ndk-env --target aarch64-linux-android
```

### Linker errors

If you get linker errors:

```bash
# Make sure the linker is set correctly
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang"
```

### Missing Android SDK

If you get "Android SDK not found" error:

```bash
# Set ANDROID_HOME explicitly
export ANDROID_HOME="/path/to/android/sdk"
```

## See Also

- [Quick Start](quick-start.md) — Get started with RustEngine
- [Architecture](architecture.md) — Engine architecture overview
- [Contributing](contributing.md) — How to contribute
