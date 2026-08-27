<!-- Purpose: Gives AI coding agents the repository workflow, architecture, and quality rules. -->

# Repository Guidelines for All LLMs / AI Agents

This file is the **entry point** for any LLM or AI coding agent (Claude Code, Gemini, Codex, Grok, Cursor, etc.) working on FunnyFileManager.

## How to Work Effectively (Lightweight RAG)

This project uses a structured, topic-based Retrieval-Augmented Generation system:

**Location:** `docs/knowledge-base/`

**Recommended workflow:**
1. Start by reading `docs/knowledge-base/00-index.md`.
2. When you need information on a topic, read the specific module file(s) from the knowledge base.
3. Modules are organized by theme (see the numbered directories).

### Most Important Modules

- **Rust Type System & Enums** (critical): `03-rust-best-practices/type-system.md`
- **Module Structure & Architecture**: `02-architecture/module-structure.md`
- **Testing Strategy**: `05-testing/testing-strategy.md`
- **Core Features** (file ops, S3, command line): `07-core-features/`
- **Far Manager Compatibility**: `09-far-manager-compatibility/hotkeys-and-parity.md`

Full index and usage guide: `docs/knowledge-base/00-index.md`

## Core Philosophy

This project places extremely high value on using Rust's type system effectively — particularly **enums for state** and **newtypes** for domain values — to make illegal states unrepresentable and produce robust code.

See `docs/knowledge-base/03-rust-best-practices/type-system.md` for details and current examples (`Overlay`, `Command`, `BookmarkSlot`, `VirtualPath`, etc.).

## Project Structure & Module Organization

- `src/main.rs`: Thin entry point - calls `funny_file_manager::run()`.
- `src/bootstrap.rs`: Production startup; loads history and creates the terminal session.
- `src/lib.rs`: Core application coordination, modal-first dispatch, and event loop.
- `src/action.rs`: Pure main-view key-to-`Action` mapping.
- `src/command.rs`, `src/lineedit.rs`, `src/complete.rs`: Command parsing, editing, history recall, and completion.
- `src/console.rs`, `src/status.rs`: Typed command transcript and user-facing outcomes.
- `src/models.rs`: Data structures (`App`, `Panel`, `DirEntry`, `Viewer`, `SortMode`).
- `src/ui.rs`: TUI rendering with ratatui.
- `src/popup.rs`: Modal input handling; depends directly on canonical operations.
- `src/ops/`: Canonical command, file, shell, viewer, and MCP-status operations.
- `src/handlers/mod.rs`: Backward-compatible re-exports; no operation implementation.
- `src/terminal.rs`: RAII lifecycle for the TUI session and external-editor suspension.
- `src/utils.rs`: Utility functions (`format_size`, `unique_destination`, `copy_dir_all`).
- `src/editor.rs`: Built-in F4 editor opening and external Alt+F4 integration.
- `src/s3.rs`: AWS S3 integration.
- `src/keepass.rs`: Optional KeePass backend (`--features keepass`).
- `mcp/`: Standalone `ffm-mcp` crate; exposes root-confined file and S3 tools over MCP stdio.
- `tests/`: Integration, mutation, coverage, and hotkey tests.
- `coverage/` and `tarpaulin-report.html`: Stored coverage artifacts (update only when intentionally refreshing coverage).
- `target/`: Cargo build output (ignored by git).

If you add major features, prefer extracting modules under `src/` and keeping `main.rs` focused on wiring and startup.

## Architecture

FunnyFileManager is a state-driven TUI with a thin binary, a library-owned
event loop, and a separate MCP server process. The root crate owns the
interactive application; `mcp/` is an independent binary and does not
participate in the TUI event loop.

### Runtime Flow

```text
src/main.rs
    -> bootstrap::run                 host terminal + persisted history
        -> lib::run_app_with_event_reader
            -> ui::ui(&App)           render the current state
            -> read KeyEvent
                -> active Overlay?    yes -> popup::process_popup_input
                |                            -> ops / editor / state transition
                no  -> action::resolve -> typed Action -> apply_action
                                                -> ops / editor / panel
            -> update App / Status / ConsoleLog -> next frame
```

Input is modal-first. An active `Overlay` consumes every key, including keys it
does not support. With no overlay, `action::resolve` is a pure key-to-`Action`
mapping; `lib.rs::apply_action` owns effect dispatch. Rendering reads `App`
after each transition and never decides which operation to run.

### State Model

- `App` is the central aggregate: two `Panel`s, the active side, view, overlay,
  lifecycle, bookmarks, status, quick search, history, and console transcript.
- `Overlay` is the single source of modal state. Its enum variants make two
  simultaneously-open dialogs unrepresentable.
- `ViewMode` (`Panels` or `Console`) is orthogonal to `Overlay`, so dialogs and
  the command line can exist over either background.
- `Lifecycle` (`Running` or `Quitting`) is the only application-exit state.
- Each `Panel` owns its selection invariant; callers use panel methods instead
  of editing the selection index directly.
- Domain newtypes and enums (`FileName`, `FileSize`, `Cells`, `ScrollTop`,
  `QuickSearch`, `Status`, and others) keep units and state transitions explicit.

### Dependency and Ownership Boundaries

```text
bootstrap -> lib -> action            pure input mapping
                  -> popup -> ops     modal input and canonical effects
                  -> ops              non-modal effects
                  -> ui               state-only rendering
                  -> editor           built-in/external editor boundary
                  -> models           shared application state

handlers ----------------> ops + popup    compatibility facade only
mcp/ -------------------------------->    separate stdio server process
```

- `src/ops/` is the sole implementation owner for command dispatch, file
  operations, shell execution, viewing, and MCP-status operations.
- `src/handlers/mod.rs` only re-exports stable paths. Do not add implementations
  there, and do not make `popup` depend on `handlers`.
- `action` performs no filesystem or terminal I/O. `ui` renders state but does
  not execute operations. `terminal` and `bootstrap` own host-terminal effects.
- The command path is `CommandLine` state -> `lineedit`/`complete` ->
  `Command::parse` -> `ops::execute_command` -> built-in operation or bounded
  shell execution. Results update typed `Status` and `ConsoleLog` values.
- Optional adapters remain behind Cargo features (`s3`, `keepass`).
  `test-helpers` exposes seams and test-only visibility, never alternate runtime
  behavior.
- `mcp/` uses its own path-validation and filesystem-operation modules. It
  confines requests to its configured root and shares only the canonical MCP
  tool-name list with the TUI status view.

### Where Changes Belong

| Change | Primary owner |
|--------|---------------|
| Main-view hotkey | `src/action.rs`, then effect dispatch in `src/lib.rs` |
| Modal/dialog key behavior | `src/popup.rs` or a focused `src/popup/` child |
| Command syntax or validation | `src/command.rs` |
| Command editing/completion/history | `src/lineedit.rs`, `src/complete.rs`, `src/history.rs` |
| File, shell, viewer, or command effect | Matching module under `src/ops/` |
| Application/domain state | `src/models.rs` or a focused `src/models/` child |
| Rendering/layout | `src/ui.rs` or a focused `src/ui/` child |
| Terminal setup/suspension/restoration | `src/bootstrap.rs`, `src/terminal.rs` |
| MCP tool or root-confined MCP filesystem behavior | `mcp/src/` |

Full module-level detail remains in
`docs/knowledge-base/02-architecture/module-structure.md`.

## Build, Test, and Development Commands

- `cargo run` (or `cargo run --bin ffm`): Run locally.
- `cargo build --release`: Build an optimized binary (output under `target/release/`).
- `cargo test`: Run the test suite.
- `cargo test --features test-helpers`: Run the full test suite, including helper-gated integration tests.
- `cargo test -- --ignored`: Run tests marked `#[ignore]` (used for manual/interactive cases).
- `cargo fmt`: Format code with `rustfmt`.
- `cargo clippy -- -D warnings`: Lint and fail on warnings.

Optional coverage (tooling may be platform-dependent): `cargo tarpaulin --out Html --output-dir coverage`.

## Rust Best Practices (Strongly Enforced)

Rules that must be followed when writing or reviewing code. This project is intentionally moving toward heavy use of enums and domain-specific types.

**Primary detailed reference**: `docs/best-practices.md` → **"Rust Type System & Modeling (Enums & Newtypes)"**

### Core Mandatory Rules

- Use `?` for error propagation. Never use `unwrap()` in library code (except tests/main).
- Prefer `&str` / `&Path` over owned types in function signatures when possible.
- **Strongly prefer enums for state** instead of multiple `bool`/`Option` fields (see the `Overlay` enum).
- Use **newtypes** instead of raw primitives for domain concepts (`BookmarkSlot`, `FileSize`, IDs, etc.).
- "Make illegal states unrepresentable" using the type system whenever feasible.
- Follow all enforced Clippy rules (`cargo clippy -- -D warnings`).
- Every `pub fn` in `src/` must have at least one test using `tempfile::TempDir` where appropriate.
- Use Conventional Commits and include verification steps in PRs.

For the complete detailed lists (Error Handling, Ownership, Collections, Structs & Enums patterns, Clippy lints, Naming, Performance, etc.), read the full sections in:
- `docs/best-practices.md`
- The older but still useful detailed list that used to live here (now consolidated in the docs)

Examples of good patterns already in the codebase: `Overlay`, `ViewMode`, `Command`, `EntryKind`,
`EditState`, `Recall`, `QuickSearch`, `DriveEntry`, `SortMode`, `PanelSide`, `VirtualPath`,
`BookmarkSlot`, `Bookmarks`, `FileSize`, `FileName`, `Cells`, `CharColumn`, `Caret`, `ScrollTop`,
`ScrollBack`, `RingIndex`, `ShellCommand`, `DurableLine`, `FailedEntry`, `Lifecycle`,
`Persistence`, `FileFormat`, `Bookmarks`. The full list, grouped by
what each buys, is in `docs/knowledge-base/03-rust-best-practices/type-system.md`.

## Commit & Pull Request Guidelines

- History mixes simple subjects and Conventional Commit style (`feat:`, `test:`, `style:`). Prefer the prefixed style for new work.
- PRs should include: a short summary plus rationale, verification steps (`cargo test`), and a screenshot or recording for UI changes. Link related issues (for example `#12`) when applicable.

## Key Architecture Decisions

- **Canonical operations**: implementations live under `src/ops/`; `handlers` is only a compatibility facade, and `popup` never depends on it.
- **Far Manager UX**: dark blue background (RGB 0,0,128), cyan text, F-key commands.
- **Dual-panel layout**: independent left/right panels; `Tab` switches the active one, `Ctrl+U` swaps contents.
- **Overlay system**: one mutually-exclusive `Overlay` enum holds all modal state — two dialogs can never be open at once.
- **Modal-first input**: an active overlay consumes every key before `Action::resolve`; unsupported command-line keys leave its buffer unchanged.
- **Input → Action DSL**: main-view keys resolve to a typed `Action` (`src/action.rs::resolve`), then `apply_action` dispatches — keymap and effects are separated and unit-testable.
- **Terminal RAII**: `TerminalSession` and `TerminalSuspension` own best-effort restoration after normal return, errors, partial transitions, and unwinding.
- **Feature-invariant runtime**: `test-helpers` exposes explicit seams and never selects alternate `run`, `main_impl`, or `handle_edit` implementations.
- **Selection preservation**: selection is kept after a panel reload.
- **Rename-with-fallback**: moves try an atomic rename, then fall back to copy+delete across filesystems.
- **Unique destinations**: never overwrites — adds a `_N` suffix (`unique_destination`).
- **Testable design**: terminal backend and event source are abstracted; injection points enable mutation testing.

## Testing

- `cargo test` — unit tests (`#[cfg(test)]`) + integration tests in `tests/`.
- `cargo test --features test-helpers` — full suite including helper-gated integration tests.
- Filesystem isolation via `tempfile::TempDir`.
- **Fault-injection points**: cfg-gated in-memory controls `FORCE_RENAME_FAILURE` (move fallback), `FORCE_COPY_DIR_ALL_STEP` (recursive-copy steps), and `FORCE_REFRESH_ERROR` (panel refresh). They are not environment variables.
- Logging tests pass explicit paths inside their own temporary directories; do not change process-wide `cwd` to redirect logs.
- Coverage: `cargo tarpaulin --out Html --output-dir coverage`. Mutation testing: `cargo mutants` (`.cargo-mutants.toml`).

## Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` | TUI framework |
| `crossterm` | Terminal handling |
| `aws-sdk-s3` / `aws-config` | AWS S3 client & config (optional, `--features s3`) |
| `tokio` | Async runtime for S3 (optional) |
| `tempfile` | Test temporary directories |
| `criterion` / `cucumber` | Benchmarks / BDD tests (dev-dependencies) |

## Detailed Reference (Knowledge Base)

Deep-dive reference lives in the knowledge base — read on demand:

- **Module reference & architecture**: `docs/knowledge-base/02-architecture/module-structure.md`
- **Far Manager hotkey parity** (full key table): `docs/knowledge-base/09-far-manager-compatibility/hotkeys-and-parity.md`
- **Type-system philosophy**: `docs/knowledge-base/03-rust-best-practices/type-system.md`
- **Testing strategy**: `docs/knowledge-base/05-testing/testing-strategy.md`

## Claude Code Slash Commands

Claude-Code-specific helpers (defined under `.claude/commands/` and `.claude/skills/`); other agents can ignore these:

| Command | What it does |
|---------|--------------|
| `/build` | Build project, fix warnings |
| `/check` | Full quality gate: fmt → clippy → build → test |
| `/clippy` | Run clippy and auto-fix all warnings |
| `/commit` | Stage, commit, rebase, push |
| `/deps` | Audit and update dependencies |
| `/doc` | Build docs and run doc tests |
| `/fix <error>` | Find and fix a specific bug |
| `/fmt` | Format all code with cargo fmt |
| `/github` | Check CI pipeline and fix failures |
| `/mutate` | Run mutation testing suite |
| `/profile` | Find performance bottlenecks |
| `/refactor <file>` | Refactor code to Rust best practices |
| `/release` | Build release binary |
| `/review` | Review recent changes for quality issues |
| `/scaffold <feature>` | Scaffold a new feature |
| `/test` | Run tests and improve coverage to 100% |
