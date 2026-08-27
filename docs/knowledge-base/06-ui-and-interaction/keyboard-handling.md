<!-- Purpose: Documents keyboard dispatch, hotkeys, and input precedence. -->

# Keyboard Handling

FunnyFileManager is designed to be fully keyboard-driven, following Far Manager conventions.

## Main Event Loop (lib.rs)

The core key dispatch lives in `run_app_with_event_reader`:

- Only `KeyEventKind::Press` events are processed (ignores Release/Repeat for Windows compatibility).
- Up/Down debounce is 250 ms in coverage builds and zero otherwise.
- `dispatch_key` first checks `app.is_overlay_active()`. If true, it sends the
  key to `popup::process_popup_input` and returns without calling
  `Action::resolve`.

## Modal Routing (`src/popup.rs`)

Only one `Overlay` variant can be active, so overlays do not compete by
priority. `process_popup_input` matches the active variant:

- Viewer (F3 / F9 reports) — arrows scroll; Esc, Enter, or F3 closes.
- Help — Esc or F1 closes.
- Editor (F4) — F2 saves; Esc or F10 closes.
- CommandLine (`:`) — Tab accepts a sole fuzzy match or opens a command/path
  dropdown for multiple matches. While the menu is visible, text edits refresh
  it, Tab and Up/Down navigate it, Enter accepts the selection, and Esc
  dismisses the menu before it can close the overlay.
- DriveSelection, DeleteConfirm, and NewFileInput
- Console (Ctrl+O) — Esc or Ctrl+O closes.

Quick search is *not* an overlay: `Alt+character` is resolved as a normal action
in `lib.rs`. There is no bookmark overlay either — see the command line below.

Every key is consumed while any overlay is active. Unsupported keys are
no-ops; they do not fall through to main-view actions. In particular,
unsupported command-line keys leave the buffer unchanged.

Command-line completion is overlay-local state. Multiple matches stay in the
dropdown rather than being reported through application status.

## Main Key Bindings (no overlay active)

**Navigation:**
- Up/Down — move selection (with wraparound)
- PgUp/PgDn — move by one panel page
- Home/End — first/last entry
- Enter — enter directory or open file
- Tab — switch active panel
- Left/Right — force select left/right panel
- Ctrl+\ — go to root

**File Operations (F-keys):**
- F1 — toggle Help
- F3 — open internal viewer
- F4 — built-in editor (Alt+F4 = external editor, Shift+F4 = create new file)
- F5 — copy
- F6 — move
- F7 — mkdir
- F8 — delete (with confirmation)
- F9 — MCP status (reuses viewer)
- F10 — quit

**Other:**
- Alt+F1 / Alt+F2 — drive selection for left/right
- Ctrl+O — toggle console mode (`ViewMode`, hides the panels and shows the
  transcript; the command line still works there)
- Ctrl+U — swap panels
- Ctrl+R — refresh panels
- ':' — open an empty command line
- any other printable key — open the command line already holding it
  (`Action::CommandLineChar`), the way Far Manager and every shell start a
  command. This is why bare `q` no longer quits: `quit`, `qemu` and `queue`
  have to be typeable.
- Alt+character — quick search (incremental; any otherwise-unclaimed character)
- Esc / Backspace — manage quick search
- Insert / Space — toggle mark + move to next
- Ctrl+C — quit

## Quick Search (Alt+character)

- The first Alt+character starts the query; later Alt+characters append.
- If a multi-character query has no match, it falls back to the last character.
- Backspace removes the last character.
- Esc and unmapped *non-printable* main-view keys resolve to `QuickSearchClear`.
  A printable key no longer clears the search — it starts a command.

## Special Windows Handling

- Only Press events are accepted to avoid double navigation.
- External commands (editors, shell fallback) use `CREATE_NO_WINDOW` flag.

## Implementation Notes for Agents

- The non-modal keymap in `action.rs` is a declarative `keymap!` macro DSL.
  Entries use Rust patterns plus an optional `if has(ALT + CONTROL)` clause.
  Modifiers use containment semantics, additional modifiers are allowed, and
  bindings are matched from top to bottom. The final wildcard keeps resolution
  total by returning `Action::QuickSearchClear`.
- Add non-modal bindings to `Action` and `action::resolve`; add modal behavior to
  the relevant `Overlay` arm in `popup::process_popup_input`.
- Prefer adding new variants to the `Overlay` enum instead of new boolean flags.
- Keep the modal-first boundary in `lib.rs`; do not bypass it for global
  shortcuts.

See `src/lib.rs` (event loop), `src/action.rs` (main-view key map), and
`src/popup.rs` (modal input).
