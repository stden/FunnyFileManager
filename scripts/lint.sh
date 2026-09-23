#!/usr/bin/env bash
# Purpose: Run the Rust linter gate used by CI, pre-commit, and `./ffm.sh lint`.
#
# Usage:
#   scripts/lint.sh              # fmt + clippy matrix + architecture contracts
#   scripts/lint.sh fmt          # format check only
#   scripts/lint.sh clippy       # clippy and architecture (all targets, all features, fallback)
#   scripts/lint.sh clippy-fast  # clippy all targets (default features)
#   scripts/lint.sh fix          # auto-format, then full clippy gate
#   scripts/lint.sh self-test    # prove readability limits and the manifest lint policy
#   scripts/lint.sh architecture # dependency ownership, layering, unsafe allowlist, file limits
#
# Exit non-zero on the first failure. Matches `.github/workflows/tests.yml` lint job.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# shellcheck disable=SC2034  # used only for messaging
mode="${1:-all}"

# Debian dropped the unversioned `python`; CI runners still ship it. Prefer
# `python3`, fall back, and fail with a readable message instead of
# "python: command not found" halfway through the gate.
PYTHON="${PYTHON:-$(command -v python3 || command -v python || true)}"
if [[ -z "$PYTHON" ]]; then
    echo "no python3 (or python) on PATH — needed by the readability checks" >&2
    exit 127
fi

run_fmt_check() {
    echo "→ cargo fmt --all -- --check"
    cargo fmt --all -- --check
    if [[ -f mcp/Cargo.toml ]]; then
        echo "→ cargo fmt --all -- --check (mcp/)"
        (cd mcp && cargo fmt --all -- --check)
    fi
    if [[ -f colorer-sys/Cargo.toml ]]; then
        (cd colorer-sys && cargo fmt --all -- --check)
    fi
}

run_fmt_fix() {
    echo "→ cargo fmt --all"
    cargo fmt --all
    if [[ -f mcp/Cargo.toml ]]; then
        echo "→ cargo fmt --all (mcp/)"
        (cd mcp && cargo fmt --all)
    fi
    if [[ -f colorer-sys/Cargo.toml ]]; then
        (cd colorer-sys && cargo fmt --all)
    fi
}

# Lint levels in Cargo.toml and thresholds in clippy.toml apply to every target.
run_clippy() {
    # Check production, tests, examples and benchmarks with the same limits.
    echo "→ cargo clippy --all-targets $* -- -D warnings"
    cargo clippy --all-targets "$@" -- -D warnings
}

run_clippy_matrix() {
    run_clippy
    # Combined features include helper-gated BDD targets and feature interactions.
    run_clippy --all-features
    # Compile the built-in highlighter without the optional Colorer adapter.
    run_clippy --no-default-features --features test-helpers
    if [[ -f mcp/Cargo.toml ]]; then
        echo "→ cargo clippy --all-targets -- -D warnings (mcp/)"
        (cd mcp && cargo clippy --all-targets -- -D warnings)
    fi
    if [[ -f colorer-sys/Cargo.toml ]]; then
        echo "→ cargo clippy --all-targets -- -D warnings (colorer-sys/)"
        (cd colorer-sys && cargo clippy --all-targets -- -D warnings)
    fi
    run_architecture
}

run_architecture() {
    echo "→ architecture contracts (ownership, dependencies, unsafe allowlist, test separation, file limits)"
    cargo test --test module_boundaries_test --test unsafe_boundary_test --test file_line_limit_test
    "$PYTHON" scripts/test_readability.py
    "$PYTHON" scripts/test_recording.py
}

# Fail in seconds, not minutes, when the compiler wrapper has stalled.
case "$mode" in
    fmt | -h | --help | help) ;;
    *) bash "$ROOT/scripts/cargo-preflight.sh" ;;
esac

case "$mode" in
    all | "")
        run_fmt_check
        run_clippy_matrix
        echo "✓ lint OK (fmt + clippy)"
        ;;
    fmt)
        run_fmt_check
        echo "✓ fmt OK"
        ;;
    clippy)
        run_clippy_matrix
        echo "✓ clippy OK"
        ;;
    clippy-fast)
        run_clippy
        echo "✓ clippy OK (default features)"
        ;;
    fix)
        run_fmt_fix
        run_clippy_matrix
        echo "✓ lint OK after format (fmt + clippy)"
        ;;
    self-test)
        "$PYTHON" scripts/test_readability.py
        # Runs Cargo on throwaway crates, hence the wrapper preflight above.
        "$PYTHON" scripts/test_lint_policy.py
        ;;
    architecture)
        run_architecture
        ;;
    -h | --help | help)
        sed -n '2,14p' "$0"
        ;;
    *)
        echo "unknown mode: $mode (try: all|fmt|clippy|clippy-fast|fix|self-test|architecture|help)" >&2
        exit 2
        ;;
esac
