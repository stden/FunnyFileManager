<!-- Purpose: Explains how Far Manager shapes the project UX and compatibility goals. -->

# 🧑‍💻 Far Manager Inspiration

FunnyFileManager is explicitly designed as a spiritual successor / modern reimplementation of Far Manager concepts in Rust.

## 🧑‍💻 What We Borrow from Far Manager

- Dual-panel interface as the primary interaction model.
- F1–F10 command bar at the bottom (with Shift/Alt/Ctrl modifiers).
- Overlays and modal dialogs (Help, Viewer, Drive selection, Delete confirmation, etc.).
- Quick search by typing Alt+letter.
- Bookmarks (0–9 slots, via `:bookmark set|go <0-9>` — no overlay, no Ctrl+digit binding).
- Drive selection (Alt+F1 / Alt+F2).
- Command line mode (':' key).
- Console mode (Ctrl+O to hide panels).
- Strong emphasis on keyboard-driven workflow.

## ↔️ Areas Where We Diverge (Intentionally)

- **Implementation language**: Rust instead of C++ / Delphi heritage.
- **Type safety**: Heavy use of enums (`Overlay`, `Command`, `DriveEntry`) and newtypes instead of raw integers and strings.
- **Testing culture**: Far more comprehensive automated testing, including mutation testing.
- **Editor strategy**: Default behavior is to delegate to external editor (VS Code, $EDITOR, nano). A full internal editor with Colorer-like highlighting is considered future work.
- **Plugin model**: Currently no plugin system (focus is on core + S3 integration). MCP server support is the modern equivalent for AI/agent extensibility.

## ⌨️ Far Manager Hotkey Compatibility

See `09-far-manager-compatibility/hotkeys-and-parity.md` for detailed mapping of which classic Far hotkeys are implemented, which are approximated, and which are intentionally different.

## 🔗 Resources

- Upstream Far Manager documentation can be fetched on demand (see `docs/far-manager/README.md` and the fetch script).
- The project maintains a living list of implemented vs. missing Far behaviors for reference.
