<!-- Purpose: Points Gemini at the shared agent guide and knowledge base. -->

# GEMINI.md

FunnyFileManager (`ffm`) is a Far Manager-style dual-panel terminal file manager written in Rust. The library owns the TUI. `mcp/` is a separate server process.

Shared rules for every agent, including Gemini, are in `AGENTS.md`. Topic detail is in `docs/knowledge-base/`. Start with `docs/knowledge-base/00-index.md`. Those two are the source of truth. This file does not keep a second architecture or testing guide.

Hotkey parity lives in `docs/knowledge-base/09-far-manager-compatibility/hotkeys-and-parity.md`. The local quality gate is `./ffm.py lint`, then `cargo test` or `cargo test --features test-helpers`.
