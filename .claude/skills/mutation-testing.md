<!-- Purpose: Documents the Claude skill for mutation-testing workflows. -->

---
name: mutation-testing
description: Use when running or writing mutation tests for FFM. Covers cargo-mutants config, mutation injection points, and mutation-killer test patterns.
---

# Mutation Testing

## Overview
FFM uses `cargo-mutants` for mutation testing to ensure tests catch real bugs, not just exercise code paths.

## Key Files
- `.cargo-mutants.toml` — Mutation testing config
- `src/mutation_test.rs` — Injection points (`FORCE_RENAME_FAILURE`, etc.)
- `src/mutation_killer_tests.rs` — Tests specifically designed to kill mutants
- `tests/handlers_mutation_tests.rs` — Handler mutation killers
- `tests/models_mutation_tests.rs` — Model mutation killers
- `tests/mutation_tests.rs` — General mutation tests

## Commands
```bash
# Run mutation tests
cargo mutants --jobs 1

# Run specific mutant
cargo mutants --mutant "replace format_size"

# Run with timeout
cargo mutants --timeout 60 --jobs 1

# Check which mutants survive
cargo mutants --list 2>&1 | head -20
```

## Injection Points
```rust
FORCE_RENAME_FAILURE    // Simulate cross-device move
FORCE_COPY_DIR_ALL_STEP // Inject error at copy step N
FORCE_REFRESH_ERROR     // Panel refresh failure
```

## Writing Mutation-Killing Tests
- Add `// Kill: replace X with Y` comment
- Test boundary conditions (0, 1, max)
- Test both branches of every if/else
- Assert exact values, not just truthiness
