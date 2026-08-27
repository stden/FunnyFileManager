<!-- Purpose: Explains the testing layers, fixtures, and coverage approach. -->

# Testing Strategy

FunnyFileManager has an unusually strong emphasis on testability and test coverage for a TUI application.

## Core Principles

- Tests must be **deterministic and non-interactive**.
- The application is designed to be testable from the ground up.
- Use the type system to make tests easier and more reliable.

## Key Testing Infrastructure

### Test Backend
- `ratatui::backend::TestBackend` is used for UI rendering tests.
- `WritableTestBackend` wrapper exists to support testing of code that expects a `Write + Backend`.

### Event Injection
- The main event loop accepts an injectable event reader (`run_app_with_event_reader`).
- This allows complete control over keyboard input during tests.

### Environment Control
- `EditorEnvGuard` and similar utilities for controlling `$EDITOR` and other environment variables in tests.
- Global locks such as `editor_env_lock` serialize tests that must modify process-wide environment state.
- Logging tests use explicit paths inside their own temporary directories and never change process-wide `cwd`.

### Feature Flags
- `test-helpers` feature enables many integration tests and test utilities.
- Many tests in `tests/` require `--features test-helpers`.

## Types of Tests

- **Unit tests** — Inside `#[cfg(test)] mod tests` in each module.
- **Integration tests** — In the `tests/` directory.
- **BDD tests** — Using Cucumber (`tests/bdd.rs` and `tests/bdd_ru.rs`) with `harness = false`.
- **Mutation testing** — Using `cargo-mutants`.
- **Coverage testing** — Using `cargo-tarpaulin` or `cargo-llvm-cov`.

## Mutation Testing

The project takes mutation testing seriously:
- `.cargo-mutants.toml` configures the tool.
- Tests often contain comments like `// Kill: replace X with Y`.
- The goal is to have very few surviving mutants.

## Coverage Goals

- High line and branch coverage is expected on new code.
- Some files are intentionally excluded (e.g., `src/main.rs`, parts of `src/editor.rs`) because they are hard or impossible to test meaningfully.
- Current target is generally ≥ 93% on testable code.

## Best Practices

- Always use `tempfile::TempDir` for filesystem operations in tests.
- Prefer focused regression tests over broad "test everything" tests.
- For UI tests, assert on rendered buffer content when possible.
- Keep heavy generated tests limited to pure functions (e.g., `format_size`).

## Recommended Commands

- Normal development: `cargo test`
- Full suite: `cargo test --features test-helpers`
- With coverage: See project scripts or `cargo tarpaulin`
- Mutation testing: `cargo mutants` (or project-specific scripts)
