---
feature: phase30-android-baseline
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 6e2d4c7..HEAD
---

# Phase 30 — Android 构建基线探测

## Report

**What was built** — Code: workspace `android-activity = { version = "0.6", features = ["game-activity"] }` so Android-target builds no longer fail on missing activity feature. Probe: rustup `aarch64-linux-android` ✅; `cargo-ndk` ✅; **NDK clang / `ANDROID_NDK_HOME` not present** on this machine — `engine-core` android builds fail at `android-activity`/`oboe-sys` native compile (expected without NDK). Docs: `docs/android-setup.md` Phase 30 baseline table + NDK/no-NDK commands; BRANCH_INDEX row 30; README 阶段 30.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-core --lib` (native) | PASS 261 |
| `cargo build -p engine-core --target aarch64-linux-android` | FAIL — ToolNotFound clang++ (缺 NDK，记录于文档） |
| rustup android target / cargo-ndk | 已安装 |

**Journey log** —
1. `android-activity` without `game-activity`/`native-activity` fails even **with** NDK — feature fix is code-side prerequisite.
2. oboe (audio default) needs NDK C++; Android 无 NDK 时建议 `--no-default-features --features unity-world-primary`（仍需 NDK 链接 android-activity）。
3. 本阶段只探测 + 文档，不安装 NDK。
4. git merge/push not handled per user preference.

## [S1] Problem
Android readiness undocumented; activity feature missing.

## [S2] Design
game-activity + probe + android-setup baseline; native tests green.

## [S3] Out of Scope
Install NDK, APK packaging, git merge/push

## Tasks

- [x] T1: android-activity game-activity + 探测记录 (covers: S2)
- [x] T2: 文档/索引 + native 测试 + finalize (covers: S2; depends: T1)
