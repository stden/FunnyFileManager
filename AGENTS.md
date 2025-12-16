# Repository Guidelines

## Project Structure & Module Organization

- `src/main.rs`: Main binary (`ffm`) containing the TUI app, panel state, and event loop.
- `tests/integration_test.rs`: Integration-style tests for key handling and terminal setup.
- `coverage/` and `tarpaulin-report.html`: Stored coverage artifacts (update only when intentionally refreshing coverage).
- `target/`: Cargo build output (ignored by git).

If you add major features, prefer extracting modules under `src/` (e.g., `src/app.rs`, `src/ui.rs`, `src/panel.rs`) and keeping `main.rs` focused on wiring and startup.

## Build, Test, and Development Commands

- `cargo run` (or `cargo run --bin ffm`): Run locally.
- `cargo build --release`: Build an optimized binary (output under `target/release/`).
- `cargo test`: Run the test suite.
- `cargo test -- --ignored`: Run tests marked `#[ignore]` (used for manual/interactive cases).
- `cargo fmt`: Format code with `rustfmt`.
- `cargo clippy -- -D warnings`: Lint and fail on warnings.

Optional coverage (tooling may be platform-dependent): `cargo tarpaulin --out Html --output-dir coverage`.

## Coding Style & Naming Conventions

- Rust 2021; rely on `rustfmt` defaults (4-space indentation).
- Naming: `snake_case` for functions/vars, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Prefer `Result` + `?` for error handling in app code; avoid unchecked `unwrap()` outside tests.

## Testing Guidelines

- Keep tests deterministic and non-interactive; use `ratatui::backend::TestBackend` where possible.
- Name tests descriptively (`test_quit_keys`, `test_navigation_keys`) and cover keybinding/UI behavior when changed.

## Commit & Pull Request Guidelines

- History mixes simple subjects and Conventional-Commit style (`feat:`, `test:`, `style:`). Prefer the prefixed style for new work.
- PRs should include: a short summary + rationale, verification steps (`cargo test`), and a screenshot/recording for UI changes. Link related issues (e.g., `#12`) when applicable.
