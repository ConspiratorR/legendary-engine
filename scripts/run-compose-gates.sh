#!/usr/bin/env bash
# Pre-merge compose gates for branch phase12-array-authority (phase 32).
# Mirrors docs/compose/BRANCH_INDEX.md. Exits 1 on first failure summary.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

FAILED=0
run_gate() {
  local name="$1"
  shift
  echo ""
  echo "=== $name ==="
  echo "$*"
  if "$@"; then
    echo "PASS $name"
  else
    echo "FAIL $name" >&2
    FAILED=$((FAILED + 1))
  fi
}

run_gate "core lib (default / unity-world-primary)" \
  cargo test -p engine-core --lib
run_gate "core lib (feature off / audio)" \
  cargo test -p engine-core --lib --no-default-features --features audio
run_gate "physics lib" \
  cargo test -p engine-physics --lib
run_gate "physics integration" \
  cargo test -p engine-physics --test physics_tests
run_gate "editor tests" \
  cargo test -p engine-editor --test editor_tests
run_gate "core examples" \
  cargo build -p engine-core --examples
run_gate "wasm engine-core" \
  cargo build -p engine-core --target wasm32-unknown-unknown --no-default-features --features unity-world-primary
run_gate "wasm engine-physics" \
  cargo build -p engine-physics --target wasm32-unknown-unknown
run_gate "wasm engine-editor lib" \
  cargo build -p engine-editor --target wasm32-unknown-unknown --no-default-features --lib
run_gate "fmt check" \
  cargo fmt -p engine-core -p engine-physics -p engine-editor --check

echo ""
echo "========== Gate summary =========="
if [ "$FAILED" -gt 0 ]; then
  echo "$FAILED gate(s) failed."
  exit 1
fi
echo "All gates passed."
exit 0
