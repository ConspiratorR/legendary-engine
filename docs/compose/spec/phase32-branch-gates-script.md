---
feature: phase32-branch-gates-script
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 34dc3cd..HEAD
---

# Phase 32 — 合并前门禁脚本

## Report

**What was built** — `scripts/run-compose-gates.ps1` / `.sh` run the BRANCH_INDEX pre-merge list. After review critical: PS1 uses `$script:results` **List.Add** so the summary/exit path is not function-local (always-exit-0 bug fixed). Scripts run all gates then `exit 1` if any failed. Smoke: injected `cmd /c exit 2` aggregates as FAIL. Parent full run: all 10 gates PASS.

**Verification**:

| Command | Result |
|---------|--------|
| `pwsh scripts/run-compose-gates.ps1` | All gates passed (10/10 in summary) |
| core lib default / opt-out | 261 / 240 |
| physics lib / physics_tests | 83 / 66 |
| editor_tests | 62 |
| wasm core/physics/editor + fmt | PASS |
| PS1 fail-closed smoke | FAIL-DETECTED + exit 1 |

**Journey log** —
1. PowerShell `$arr +=` inside functions is script-local — use `$script:list.Add`.
2. Gate scripts must fail-closed (summary + non-zero exit), not only print FAIL.
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
