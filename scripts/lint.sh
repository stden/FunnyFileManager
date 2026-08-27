#!/usr/bin/env bash
# Purpose: Run the Rust linter gate used by CI, pre-commit, and `./ffm.sh lint`.
#
# Usage:
#   scripts/lint.sh              # full gate (fmt + clippy feature matrix)
#   scripts/lint.sh fmt          # format check only
#   scripts/lint.sh clippy       # clippy only (production + test-helpers + keepass)
#   scripts/lint.sh clippy-fast  # clippy production only (no optional features)
#   scripts/lint.sh fix          # auto-format, then full clippy gate
#
# Exit non-zero on the first failure. Matches `.github/workflows/tests.yml` lint job.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# shellcheck disable=SC2034  # used only for messaging
mode="${1:-all}"

run_fmt_check() {
    echo "→ cargo fmt --all -- --check"
    cargo fmt --all -- --check
    if [[ -f mcp/Cargo.toml ]]; then
        echo "→ cargo fmt --all -- --check (mcp/)"
        (cd mcp && cargo fmt --all -- --check)
    fi
}

run_fmt_fix() {
    echo "→ cargo fmt --all"
    cargo fmt --all
    if [[ -f mcp/Cargo.toml ]]; then
        echo "→ cargo fmt --all (mcp/)"
        (cd mcp && cargo fmt --all)
    fi
}

# Three clippy invocations mirror CI:
# 1. no features     — production cfg paths
# 2. test-helpers    — test seams and conditional test code
# 3. keepass         — off-by-default kdbx module (not parsed otherwise)
# 4. s3              — off-by-default AWS module
run_clippy() {
    local features="${1:-}"
    # `--all-targets` covers lib, bins, tests, examples, and benches so
    # pedantic noise in tests cannot silently regress.
    if [[ -z "$features" ]]; then
        echo "→ cargo clippy --all-targets -- -D warnings"
        cargo clippy --all-targets -- -D warnings
    else
        echo "→ cargo clippy --all-targets --features ${features} -- -D warnings"
        cargo clippy --all-targets --features "$features" -- -D warnings
    fi
}

run_clippy_matrix() {
    run_clippy
    run_clippy "test-helpers"
    run_clippy "keepass"
    run_clippy "s3"
    if [[ -f mcp/Cargo.toml ]]; then
        echo "→ cargo clippy --all -- -D warnings (mcp/)"
        (cd mcp && cargo clippy --all -- -D warnings)
    fi
}

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
        echo "✓ clippy OK (production only)"
        ;;
    fix)
        run_fmt_fix
        run_clippy_matrix
        echo "✓ lint OK after format (fmt + clippy)"
        ;;
    -h | --help | help)
        sed -n '2,14p' "$0"
        ;;
    *)
        echo "unknown mode: $mode (try: all|fmt|clippy|clippy-fast|fix|help)" >&2
        exit 2
        ;;
esac
