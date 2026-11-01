<!-- Purpose: Lean agent entry point — rules, commands, and pointers. Full reference: AGENTS_FULL.md -->

# FunnyFileManager — Agent Entry Point

Stack: Rust 1.95 · ratatui · crossterm. `mcp/` ships the `ffm-mcp` server (63 tools) as a separate process.

**Full reference (architecture, module map, Rust rules, testing, commit guide):** → [`AGENTS_FULL.md`](AGENTS_FULL.md)

## Start Here

1. Read `docs/knowledge-base/00-index.md` for topic navigation.
2. Key modules on demand: `02-architecture/`, `03-rust-best-practices/`, `05-testing/`, `09-far-manager-compatibility/`.

## Build & Test

```bash
cargo build --release               # optimized binary → target/release/
cargo run                           # dev run
cargo test                          # unit + integration tests
cargo test --features test-helpers  # include helper-gated tests
cargo clippy -- -D warnings         # lint (fail on warnings)
cargo fmt                           # format
./ffm.py lint                       # full gate: typos + clippy + fmt
./ffm.py coverage                   # line-coverage gate (≥98%)
```

## Core Rules (non-negotiable)

- No `unwrap()`/`expect()` outside tests — use `?` for error propagation.
- No `mod.rs` files — use `foo.rs` + `foo/` (Rust 2018 layout; `mod_module_files` enforces it).
- No file over 200 lines — `tests/file_line_limit_test.rs` enforces it.
- `pub(crate)` by default; `pub` only for what `lib.rs` re-exports or `tests/` exercises.
- Enums for state, newtypes for domain values — make illegal states unrepresentable.
- TDD: write the failing test first, then implement the smallest complete change.
- Run `./ffm.py lint` + `cargo test` before every commit.
- Commit with Conventional Commits (`feat:`, `fix:`, `chore:`, etc.).
- Push to `main` after pulling (`git pull --rebase`) and passing the quality gate.

## Do NOT Touch

- `coverage/` — stored artifacts; update only when refreshing coverage intentionally.
- `target/` — Cargo build output (git-ignored).
- `rust-toolchain.toml` — bump in its own commit only.
- Complexity thresholds in `clippy.toml` — do not raise them.
- `tests/module_boundaries_test.rs` — any file it names is a two-file change (file + test).

## Where Changes Belong (quick map)

| Change | Owner |
|--------|-------|
| Main-view hotkey | `src/action.rs` → `src/application/dispatch.rs` |
| Editor key / command | `src/action/editor.rs` → `src/popup/editor.rs` |
| File / shell / viewer effect | `src/ops/<module>` |
| Application / domain state | `src/models.rs` |
| Rendering / layout | `src/ui.rs` |
| MCP tool | `mcp/src/` |

Full detail → [`AGENTS_FULL.md`](AGENTS_FULL.md)
