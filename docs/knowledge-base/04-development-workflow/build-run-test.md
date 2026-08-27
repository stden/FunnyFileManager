<!-- Purpose: Lists the commands used to build, run, and test the project. -->

# Build, Run, Test, and Coverage

## Essential Commands

| Command | Purpose |
|---------|---------|
| `cargo run` (or `cargo run --bin ffm`) | Run locally in debug mode |
| `cargo build` | Debug build |
| `cargo build --release` | Optimized release binary |
| `cargo test` | Run unit + basic tests |
| `cargo test --features test-helpers` | Full test suite (recommended for most development) |
| `cargo test -- --ignored` | Run tests marked `#[ignore]` |
| `cargo fmt` | Format all code |
| `cargo clippy -- -D warnings` | Lint (must be clean) |
| `./scripts/lint.sh` / `./ffm.sh lint` | Full gate: fmt check + clippy feature matrix (matches CI) |
| `cargo tarpaulin --out Html --output-dir coverage` | Generate HTML coverage report |

## Important Environment Details

- The project uses `.cargo/config.toml` with `jobs = 1` by default to avoid OOM when compiling heavy dependencies (especially the AWS SDK crates).
- Many integration and UI tests are gated behind the `test-helpers` feature.
- BDD tests (`tests/bdd.rs` and `tests/bdd_ru.rs`) use `harness = false` (Cucumber-based) and require both `test-helpers` and `s3` features.

## Coverage Expectations

- High coverage is expected on new logic.
- Some files are intentionally excluded from coverage targets (`src/main.rs`, parts of `src/editor.rs`) because they are difficult or impossible to test meaningfully in an automated way.
- Current practical target is generally ≥ 93% on testable code.

## Mutation Testing

The project uses `cargo-mutants` (configured via `.cargo-mutants.toml`).

- Tests frequently contain comments like `// Kill: replace X with Y`.
- Running mutation testing is encouraged on changed code.
- The goal is very few surviving mutants on core logic.

## Recommended Daily Workflow

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test --features test-helpers
```

For deeper validation:
- Run mutation testing on modified modules.
- Generate coverage report periodically.

**Even better:** Install the pre-commit hook (see git-and-commits.md). It will run `cargo fmt` automatically on every `git commit`.

## Notes for AI Agents

When making code changes, always verify with at least:
- `cargo fmt`
- `cargo clippy -- -D warnings`
- Relevant tests (preferably with `--features test-helpers`)

If the pre-commit hook is installed in the environment, formatting will be applied automatically before the commit lands.