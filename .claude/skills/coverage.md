<!-- Purpose: Documents the Claude skill for coverage-focused testing work. -->

---
name: coverage
description: Use when checking or improving test coverage for FFM. Covers cargo-tarpaulin, coverage reports, and targeted test writing.
---

# Test Coverage

## Overview
FFM targets 100% test coverage using `cargo-tarpaulin`.

## Commands
```bash
# Run coverage
cargo tarpaulin --out html --output-dir coverage/

# Quick coverage report
cargo tarpaulin --print-rust-only 2>&1 | tail -20

# Coverage with test helpers
cargo tarpaulin --features test-helpers --out html

# View report
open coverage/tarpaulin-report.html
```

## Coverage Test Files
- `tests/coverage_100_test.rs` — Final push to 100%
- `tests/coverage_gaps_test.rs` — Filling identified gaps
- `tests/coverage_smoke_test.rs` — Smoke coverage
- `tests/extra_coverage_test.rs` — Extra edge cases
- `tests/increased_coverage_test.rs` — Incremental additions
- `tests/targeted_coverage_tests.rs` — Targeted coverage

## Config
`tarpaulin.toml` at project root.

## Tips
- Use `#[cfg(tarpaulin)]` for coverage-specific code paths
- Check `coverage.txt` for current baseline
- Every `pub fn` in `src/` must have at least one test
