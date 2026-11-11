<!-- Purpose: Points Claude Code at the shared agent guide and knowledge base. -->

# CLAUDE.md

FunnyFileManager (`ffm`) is a Far Manager-style dual-panel terminal file manager. The Rust library owns the TUI. `mcp/` is a separate server process.

Shared rules for every agent are in `AGENTS.md`. Topic detail is in `docs/knowledge-base/`. Start with `docs/knowledge-base/00-index.md`. Those two are the source of truth. This file only says where to read them, so it does not keep a second copy of the module map or the hotkey table.

Hotkey parity lives in `docs/knowledge-base/09-far-manager-compatibility/hotkeys-and-parity.md`.

Claude Code slash commands are the prompts under `.claude/commands/`. The local quality gate is `./ffm.py lint`, then the test command the change needs (`cargo test`, or `cargo test --features test-helpers` when the test uses those helpers).

<!-- rtk-instructions v2 -->
# Command output

Command output here is condensed to save tokens, keeping every signal and
dropping costly noise. Treat it as the complete result: run commands
normally, and batch related commands into one call to avoid extra turns.
Truncated results state their recovery path in their own output. Re-run a
command as `rtk proxy <cmd>` only when its result is unusable: empty when
output was clearly expected, contradicting its exit code, or garbled.
<!-- /rtk-instructions -->
