<!-- Purpose: Explains the test-driven workflow expected for Rust changes. -->

# Testing and TDD

Ship behavior with tests. The detailed layout of suites, seams, and fixtures is `docs/knowledge-base/05-testing/testing-strategy.md`. This page is the short version.

## Red, green, refactor

For a behavior change or a bug fix:

1. Write a failing test that names the behavior.
2. Make the smallest change that passes.
3. Clean up while the tests stay green.

A bug fix starts from a regression test that fails on the broken behavior.

## Where tests live

- Unit tests sit in a `tests.rs`, `tests_*.rs`, or `tests/` child of the module that owns the behavior. They are not inline `mod tests` blocks in a production file.
- Integration tests live in `tests/`.
- Cucumber scenarios live under `tests/features/` and `tests/features_ru/`. Their harness is `harness = false`, so `cargo test --test bdd` runs them. `cargo nextest` cannot list them, and its default filter (`.config/nextest.toml`) leaves them out.
- Filesystem tests use `tempfile::TempDir`. Build panels with `App::with_panels` or `Panel::from_entries`. `App::new()` lists the real working directory.
- Fixture paths use `/home/bob`, `/srv/x`, and `example.com`.

## Commands

```bash
cargo test --features test-helpers
./ffm.py lint
cargo test --features test-helpers,s3 --test bdd --test bdd_ru
```

CI measures line coverage with `cargo llvm-cov`: 98% of the measured root set, 75% for the MCP crate. Run the root gate locally with `./ffm.py coverage`; `scripts/coverage.py` holds its features, filename exclusions and floor, and CI runs the same script. A shorter command measures a different set and is not that gate.

Mutation testing is `cargo mutants`, scoped by `.cargo/mutants.toml`. A mutation-named test file is not evidence that mutants were run. Record caught, missed, unviable, timed-out, and excluded counts separately.
