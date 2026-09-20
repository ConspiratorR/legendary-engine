---
feature: phase32-branch-gates-script
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 34dc3cd..HEAD
---

# Phase 32 — 合并前门禁脚本

## Report

**What was built** — `scripts/run-compose-gates.ps1` and `scripts/run-compose-gates.sh` execute the BRANCH_INDEX pre-merge checklist (core dual-mode tests, physics lib+integration, editor tests, examples build, wasm core/physics/editor-lib, fmt --check). Script prints PASS/FAIL per gate and exits non-zero on any failure. BRANCH_INDEX and README document the one-liner.

**Verification**:

| Command | Result |
|---------|--------|
| `pwsh scripts/run-compose-gates.ps1` | **All gates passed** |
| core lib default / opt-out | 261 / 240 |
| physics lib / physics_tests | 83 / 66 |
| editor_tests | 62 |
| wasm core / physics / editor-lib | PASS |
| fmt --check | PASS |

**Journey log** —
1. One script mirrors BRANCH_INDEX so merge checklist is executable.
2. Feature-off core uses `--no-default-features --features audio`.
3. git merge/push not handled per user preference.

## [S1] Problem
Manual gate list easy to skip/mis-run.

## [S2] Design
PS1 + sh scripts + docs; same commands as BRANCH_INDEX.

## [S3] Out of Scope
CI yaml changes, Android NDK, git merge/push

## Tasks

- [x] T1: 双平台门禁脚本 (covers: S2)
- [x] T2: 文档 + finalize (covers: S2; depends: T1)
