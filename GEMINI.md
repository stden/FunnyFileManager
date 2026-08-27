<!-- Purpose: Gives Gemini repository-specific workflow and quality instructions. -->

# GEMINI.md

This file provides a comprehensive overview of the **FunnyFileManager (ffm)** project, intended to serve as a guide for future AI-driven development and analysis.

---

## Instructions for AI Agents (Gemini and other LLMs)

This project uses a lightweight shared **Knowledge Base (RAG)** located at:

**`docs/knowledge-base/`**

**Recommended workflow:**
1. Start by reading `docs/knowledge-base/00-index.md`.
2. Read the specific topic module(s) you need.
3. The modules are organized by theme for efficient retrieval.

Particularly important:
- `03-rust-best-practices/type-system.md` (core philosophy around enums and newtypes)
- `02-architecture/module-structure.md`
- `05-testing/testing-strategy.md`

See `AGENTS.md` for the complete instructions on using the knowledge base (applies to all LLMs).

---

## Project Overview

**ffm** is a dual-panel, terminal-based file manager written in Rust, inspired by the functionality of Far Manager. It leverages the `ratatui` and `crossterm` libraries to create a rich, interactive terminal user interface.

### Architecture

The project follows a modular and well-structured architecture, promoting a clear separation of concerns:

-   **`src/main.rs`**: Thin binary entry point that calls `funny_file_manager::run()`.
-   **`src/lib.rs`**: Core coordination and `run_app_with_event_reader`. It routes every key to an active overlay before resolving main-view actions.
-   **`src/models.rs`**: Defines the primary data structures that represent the application's state. This includes the main `App` struct, the `Panel` struct for each side of the file manager, `DirEntry` for individual file/directory items, and various state-related enums like `SortMode`.
-   **`src/ui.rs`**: Responsible for all UI rendering using `ratatui`. It constructs the layout, draws the panels, status bar, command bar, and any overlays like the help window or file viewer.
-   **`src/action.rs`**: Pure main-view `KeyEvent` to `Action` mapping.
-   **`src/popup.rs`**: Modal input and overlay transitions. Active overlays consume every key and call canonical operations directly.
-   **`src/ops/`**: Sole implementation owner for command dispatch, file operations, shell execution, file viewing, and MCP status.
-   **`src/handlers/mod.rs`**: Backward-compatible re-export facade with no production operation implementation.
-   **`src/terminal.rs`**: RAII lifecycle for the application terminal session and temporary suspension around an external editor.
-   **`src/editor.rs`**: Built-in F4 editor opening and external Alt+F4 process integration.
-   **`src/utils.rs`**: Provides miscellaneous utility functions used across the application.
-   **`/tests`**: Contains integration and behavior-driven tests, demonstrating a commitment to code quality.

## Building and Running

The project uses the standard Rust toolchain (`cargo`).

-   **Build (Debug)**:
    ```bash
    cargo build
    ```
-   **Build (Release)**:
    ```bash
    cargo build --release
    ```
-   **Run**:
    ```bash
    cargo run
    ```
-   **Run Tests**:
    ```bash
    cargo test
    ```
-   **Test Coverage**: The project is configured for code coverage analysis using `cargo-tarpaulin`.
    ```bash
    cargo tarpaulin --out html
    ```
    This will generate a report in `./tarpaulin-report.html`.

-   **Install**: To install the `ffm` binary to your local cargo binary path (`~/.cargo/bin/`):
    ```bash
    cargo install --path .
    ```

## Development Conventions

-   **Rust Type System Philosophy** (Very Important):
    - The project is actively migrating toward richer domain modeling using **enums for state** and **newtypes** instead of raw primitives and scattered boolean/Option flags.
    - See the new `Overlay` enum, `Command` enum, `BookmarkSlot` newtype, and guidance in `docs/best-practices.md`.
    - When designing new features or refactoring, prefer making illegal states impossible to represent at the type level.

-   **Testing Strategy**: The codebase places a high value on testability.
    -   Unit tests are located within their respective modules under a `#[cfg(test)]` block.
    -   Integration tests are located in the `tests/` directory.
    -   The application is designed for testing by abstracting the terminal backend (`TestBackend`) and event source, allowing key presses and UI output to be simulated.
    -   The `test-helpers` feature exposes explicit utilities such as `WritableTestBackend` and `handle_edit_test_mode`; it never selects alternate `run`, `main_impl`, or `handle_edit` implementations.
    -   File-operation faults use cfg-gated in-memory controls, not environment variables.
    -   Logging tests pass explicit temporary paths instead of changing process-wide `cwd`.

-   **Terminal Safety**: `TerminalSession` and `TerminalSuspension` conservatively roll back partial transitions and continue best-effort cleanup after an individual failure.

-   **Error Handling**: The application uses standard Rust `Result` types for error propagation. When runtime logging is enabled, top-level errors are recorded in `ffm_errors.log`.

-   **Logging**: Runtime logging is disabled by default. With `FFM_LOG=1`, payload-redacted actions and errors are written to `ffm_actions.log` and `ffm_errors.log` in the system temporary directory; Unix files are created owner-only.

-   **Mutation Testing**: The presence of `.cargo-mutants.toml` and related files indicates that the project uses mutation testing via `cargo-mutants` to ensure the quality and robustness of its test suite.

-   **UI Library**: The UI is built with `ratatui`, a popular library for creating terminal interfaces in Rust. The code in `src/ui.rs` demonstrates the use of widgets, layouts, and styling to create the application's interface.

For complete coding standards that apply to all AI agents, refer to:
- `AGENTS.md` (primary guidelines for any LLM)
- `docs/best-practices.md` (especially the Rust Type System section)
