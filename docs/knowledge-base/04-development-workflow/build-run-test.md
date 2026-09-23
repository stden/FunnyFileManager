<!-- Purpose: Lists the commands used to build, run, and test the project. -->

# 🏗️ Build, Run, Test, and Coverage

## 💬 Essential Commands

| Command | Purpose |
|---------|---------|
| `cargo run` (or `cargo run --bin ffm`) | Run locally using the incremental development profile |
| `cargo build` | Build the development binary for fast iteration |
| `cargo build --release` | Optimized release binary |
| `cargo test` | Run unit + basic tests |
| `cargo test --features test-helpers` | Include helper-gated integration tests |
| `cargo test -- --ignored` | Run tests marked `#[ignore]` |
| `cargo fmt` | Format all code |
| `cargo clippy -- -D warnings` | Lint (must be clean) |
| `./scripts/lint.sh` / `./ffm.sh lint` | Shared local/CI format and Clippy gate for TUI, MCP, and Colorer |
| `./ffm.sh lint self-test` | Compile positive/negative readability fixtures and compare behavior; requires Python |
| `cargo build --manifest-path mcp/Cargo.toml` | Build the standalone MCP server with its development profile |
| `cargo test --manifest-path mcp/Cargo.toml` | Run the MCP package's separate test suite |
| `cargo tarpaulin --out Html --output-dir coverage` | Generate HTML coverage report |

The shared lint gate checks formatting in the root and MCP packages, then
runs Clippy with `--all-targets -- -D warnings` for the root's default features,
all features, and `--no-default-features --features test-helpers`. It also
checks every target in MCP and Colorer. This covers helper-gated tests, feature
interactions, and the built-in highlighter without the optional Colorer adapter.
CI calls the same script, then runs `self-test` and architecture contracts as
separate checks. The self-test can also run directly as
`python scripts/test_readability.py`; it invokes the compilers without Cargo.

## ⚙️ Important Environment Details

- `rust-toolchain.toml` selects stable Rust with rustfmt and Clippy for all
  packages, matching CI. The manifests retain Rust 1.95 as the minimum version.
- Cargo's job count is intentionally unset, so local builds use every logical
  CPU by default. Override it with `cargo build -j N` or `CARGO_BUILD_JOBS` on a
  memory-constrained machine.
- This project leaves compiler caching opt-in. `.cargo/config.toml` sets an
  empty `rustc-wrapper`, which switches off a wrapper forced by a user or
  managed Cargo configuration: a shared sccache server that stalls keeps every
  build waiting for minutes with no `rustc` running. Developers enable a cache
  per shell with `RUSTC_WRAPPER=sccache`, which outranks the file. Do not set
  `SCCACHE_NO_DAEMON`: a foreground server inherits Cargo's output pipes and the
  build never finishes. `cargo llvm-cov` cannot run an empty wrapper, so
  coverage runs set `CARGO_BUILD_RUSTC_WRAPPER=/usr/bin/env`, as CI does;
  `ffm.cmd coverage` uses Git for Windows' `env.exe` the same way.
  `scripts/cargo-preflight.sh` (run by `./ffm.sh` and `scripts/lint.sh`) asks
  the selected wrapper for `rustc -vV` and fails within
  `FFM_WRAPPER_PROBE_SECS` (default 20) seconds instead of hanging; without GNU
  `timeout` (or `gtimeout`) it skips the probe. Cargo
  incremental compilation remains enabled for both development and test
  profiles, unless a user config sets `build.incremental = false`, which
  overrides every profile.
- Plain `cargo build` and `cargo run` compile project code without optimization
  or LTO, with incremental compilation and 256 codegen units. Debug information
  is disabled and stripped; dependencies use optimization level 2. These
  settings avoid full application optimization and cross-crate linking work
  during routine edits. Enable debug information explicitly if a debugger
  needs source-level symbols.
- `cargo build --release` keeps optimization level 3, fat LTO, a single
  codegen unit, stripped symbols, and aborting panics for production binaries.
  It trades build time for runtime optimization. The root crate also provides
  `cargo build --profile release-fast`, which uses thin LTO and 16 codegen units
  for an intermediate option.
- The root and `mcp/` packages use aligned development and test profiles. MCP
  retains Cargo's release defaults. Both packages, including `colorer-sys`,
  share the root `target/` directory through `.cargo/config.toml`; dependencies
  can reuse artifacts when versions, features, and profiles match. Their
  lockfiles remain independent and tracked. Root-crate commands do not build
  or test MCP automatically.
- Windows uses `lld-link`; Linux uses `mold`. These linker settings live in
  `.cargo/config.toml` and apply without changing the commands below.
- Many integration and UI tests are gated behind the `test-helpers` feature.
- BDD tests (`tests/bdd.rs` and `tests/bdd_ru.rs`) use `harness = false` (Cucumber-based) and require both `test-helpers` and `s3` features.

## 🧪 Coverage Expectations

- High coverage is expected on new logic.
- Some files are intentionally excluded from coverage targets (`src/main.rs`, parts of `src/editor.rs`) because they are difficult or impossible to test meaningfully in an automated way.
- CI enforces 98% measured line coverage for the TUI coverage job and 75% for
  the MCP package. Exclusions and feature sets are explicit in
  `.github/workflows/tests.yml`; these are not whole-repository coverage claims.

## 🧪 Mutation Testing

The project uses `cargo-mutants`, configured via `.cargo/mutants.toml` (the
only path the tool reads; it selects `cargo-nextest` as the test runner).

- Tests frequently contain comments like `// Kill: replace X with Y`.
- Running mutation testing is encouraged on changed code.
- The goal is very few surviving mutants on core logic.

## ✅ Recommended Daily Workflow

```bash
cargo fmt -- --check
cargo test
cargo test --features test-helpers
./ffm.sh lint
```

For deeper validation:
- Run mutation testing on modified modules.
- Generate coverage report periodically.

**Even better:** Install the pre-commit hook (see git-and-commits.md). It will run `cargo fmt` automatically on every `git commit`.

## 📝 Notes for AI Agents

When making code changes, always verify with at least:
- `cargo fmt`
- `./ffm.sh lint`
- Relevant tests (preferably with `--features test-helpers`)

If the pre-commit hook is installed in the environment, formatting will be applied automatically before the commit lands.
