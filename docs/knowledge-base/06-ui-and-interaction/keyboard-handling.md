<!-- Purpose: Documents keyboard dispatch, hotkeys, and input precedence. -->

# ⌨️ Keyboard Handling

FunnyFileManager is designed to be fully keyboard-driven, following Far Manager conventions.

## 🔁 Main Event Loop (`src/application/mod.rs`)

`run_app_with_wsl_backend` owns rendering, event reads, and WSL worker pumping
in `src/application/mod.rs`. Production hands out keys crossterm has already
queued at once and waits up to 100 ms only when none are queued; the existing
`run_app_with_event_reader` adapter preserves the event-only injection API.
Keyboard and mouse dispatch live in `src/application/dispatch.rs`:

- Only `KeyEventKind::Press` events are processed (ignores Release/Repeat for Windows compatibility).
- Keys typed while one event kept the screen from answering for 300 ms or more
  (a long copy, a delete, an archive, a `:` command) are thrown away, not run:
  they were typed at a frozen screen. `terminal::discard_queued` drains them
  unrecorded, so a replay does not run them either; `terminal::is_cancel_event`
  is the one rule for the Esc / Ctrl+C that a running `:` command still obeys.
- Live key events pass `terminal::decode_input` first, in `recording::Input`,
  so a recording holds the decoded key. Where the terminal speaks the kitty
  keyboard protocol FFM pushes it at startup and keys arrive unambiguous;
  otherwise the one legacy byte two bindings share (0x1C: Ctrl+\ and Ctrl+4)
  is taken as Ctrl+\. See the parity doc's "Terminal key encoding".
- `dispatch_key` first checks `app.is_overlay_active()`. If true, it sends the
  key to `popup::process_popup_input` and returns without calling
  `action::resolve`.
- With no overlay, a panel that is not a file list — a tree, a quick view,
  NetBox's sessions list or a remote host — is asked about the key first
  (`panel_kind_consumes`). The two NetBox panels claim their list keys and
  their own F-keys; any other key is resolved, and when `Action::scope()`
  says it acts on the active panel's `Entries` it is refused with a status
  line rather than acting on the file list hidden beneath. `Program` keys
  (Tab, Alt+F1/F2, F9–F12, the command line) fall through. F9 menu rows run
  through `ops::menu::activate` are asked the same through `menu_scope`.

## 🪟 Modal Routing (`src/popup.rs`)

Only one `Overlay` variant can be active, so overlays do not compete by
priority. `process_popup_input` matches the active variant:

- Viewer (F3 / F11 reports) — arrows or the mouse wheel scroll; Esc, Enter, or F3 closes.
- Help — Esc or F1 closes.
- Editor (F4) — `popup/editor.rs` handles search and save confirmation first,
  then interprets typed commands from `action/editor.rs`; F2 saves, and Esc or
  F10 closes.
- CommandLine (`:`) — Tab accepts a sole fuzzy match or opens a command/path
  dropdown for multiple matches. While the menu is visible, text edits refresh
  it, Tab and Up/Down navigate it, Enter accepts the selection, and Esc
  dismisses the menu before it can close the overlay.
- DriveSelection, DeleteConfirm, and NewFileInput
- WSL manager — `action/wsl.rs` maps keys to `WslCommand`; `popup/wsl.rs`
  interprets navigation and queues operations. Enter opens the selected root,
  F2 refreshes distributions, F5 updates WSL, F6 updates selected-distribution
  packages, and Esc closes. Busy operations still allow navigation and close.

Quick search is *not* an overlay: `Alt+character` is resolved as a normal action
by `action::resolve`, then applied in `application/dispatch.rs`. Console is the
background `ViewMode`, not an overlay, so the command line remains usable over
it. There is no bookmark overlay either — see the command line below.

Every key is consumed while any overlay is active. Unsupported keys are
no-ops; they do not fall through to main-view actions. In particular,
unsupported command-line keys leave the buffer unchanged.

Command-line completion is overlay-local state. Multiple matches stay in the
dropdown rather than being reported through application status.

## ✏️ Editor Selection and Clipboard

- Shift with Left/Right/Up/Down, Home/End, or PageUp/PageDown starts or extends
  a stream selection. Movement without Shift clears it.
- Ctrl+C or Ctrl+Insert copies selected text; Shift+Insert pastes and replaces any
  selection. Typing or Enter replaces selected text; Backspace/Delete removes it.
- Ctrl+F, as in Far, types the edited file's full name at the cursor (replacing
  any selection) and copies it to the clipboard; a failed copy is reported, and
  the name is typed all the same.
- Selection positions use characters, while rendering converts them to terminal
  cells. Model transitions preserve Unicode and normalize selection direction.
- `ops/editor/clipboard.rs` owns clipboard effects through the session's
  injected backend. Copy prefers the native clipboard and can fall back to
  OSC 52; paste reads native text. Failed clipboard access leaves the buffer
  and selection intact and reports an editor-bar notice.
- Search and unsaved-change confirmation remain modal within the editor;
  selection/clipboard shortcuts cannot escape those dialogs.

## ✏️ Text Fields: Selection and Clipboard

Every one-line text field of a dialog is a `LineEditor` (`src/lineedit.rs`,
selection in `lineedit/selection.rs`) and hands its editing keys to one
handler, `popup/text_field.rs`, so the keys mean the same everywhere:

- Shift with Left/Right/Home/End selects; Ctrl+Shift+Left/Right by word;
  Ctrl+A selects the whole field. A move without Shift drops the selection.
- Typing, a paste, Backspace and Delete replace or remove the selection.
- Ctrl+C / Ctrl+Insert copy it, Ctrl+X / Shift+Delete cut it, Ctrl+V /
  Shift+Insert paste (as one line: line breaks become spaces, a trailing one
  is dropped), through the session's clipboard backend as the editor's copy
  does.
- A terminal paste (bracketed paste) types into the field under the cursor
  (`App::focused_text_field`).
- `dialog::Row::line` / `Row::field` draw a field from its `LineEditor`: the
  caret where it is, the selection in the selected-field colours.

Fields on it: rename / make folder / link name and target (Shift+F6, F7,
Alt+F6 — the name opens selected), the F5/F6 destination, Shift+F4 new file,
Alt+F7 mask and text, Shift+F1 archive name, Shift+F2 destination, the Gray
+/- mask (its offer opens selected), Ctrl+G, Ctrl+Z describe, the F9 prompts,
the PATH and language-server editors and the task list's bars and priority.
The command line takes the same selection and clipboard keys, except that
Ctrl+C with nothing selected still clears the line and Ctrl+A still goes to
its start (readline). Still plain text with the caret at the end: the
editor's search prompt, the NetBox session fields, the attributes dialog's
time fields and the KeePass forms.

## 🪟 Main Key Bindings (no overlay active)

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
- F2 — open the available user-command menu
- F3 — open internal viewer
- F4 — built-in editor (Alt+F4 = external editor, Shift+F4 = create new file)
- F5 — copy
- F6 — move
- F7 — mkdir
- F8 — delete (with confirmation)
- F9 — open the menu bar
- F10 — quit
- F11 — Far's Plugin commands list: MCP status (reuses viewer), NetBox's stored sessions, the NetBox `.netbox` import
- F12 — switch between panels and console

**Other:**
- Alt+F1 / Alt+F2 — drive selection for left/right
- Ctrl+O — toggle console mode (`ViewMode`, hides the panels and shows the
  transcript; the command line still works there)
- Ctrl+U — swap panels
- Ctrl+R — refresh panels
- Ctrl+Alt+W — open the WSL manager on Windows (also `wsl` or F9 → Commands)
- ':' — open an empty command line
- any other printable key — open the command line already holding it
  (`Action::CommandLineChar`), the way Far Manager and every shell start a
  command. This is why bare `q` no longer quits: `quit`, `qemu` and `queue`
  have to be typeable.
- Alt+character — quick search (incremental; any character, including `-`,
  `+` and `*`: only the bare keys open the group-selection dialogs)
- Esc / Backspace — manage quick search
- Insert / Space — toggle mark + move to next
- Ctrl+C — quit

## 🖱️ Mouse Routing

- A single left click on a panel row selects that entry and activates its panel.
- If the command line is active, clicking a visible panel closes the unfinished
  prompt and gives that panel focus; floating dialogs remain fully modal.
- A double-click on the same entry performs the same `Action::Open` as Enter.
  This enters directories, activates `..` to leave a directory, and opens files
  without introducing a mouse-only operation path.
- Clicking the function-key bar dispatches the corresponding F-key.
- Shift/Alt/Ctrl-click on the function-key bar preserves the modifier.
- The mouse wheel moves the selection in the panel under the pointer, activates
  that panel, and scrolls viewers, menus, dialogs, and the console through the
  same Up/Down paths as the keyboard.
- Menu entries and enabled dialog buttons run the action named by the control,
  regardless of which control previously held keyboard focus.
- An active overlay consumes clicks, so controls behind a dialog cannot run.

## 🔍 Quick Search (Alt+character)

- The first Alt+character starts the query; later Alt+characters append.
- If a multi-character query has no match, it falls back to the last character.
- Backspace removes the last character.
- Any other key ends the search before it acts, as Far's fast find closes on
  any key that is not part of it: Enter, Tab, the arrows, Home/End, PgUp/PgDn,
  Insert and a printable key (which starts a command) all do.
  `application/dispatch.rs::apply_action` clears the query for every action
  that is not `Action::edits_quick_search`, so a query never follows the user
  into another folder or panel. Esc and unmapped keys resolve to
  `QuickSearchClear`.

## 🪟 Special Windows Handling

- Only Press events are accepted to avoid double navigation.
- External commands (editors, shell fallback) use `CREATE_NO_WINDOW` flag.

## ✏️ Implementation Notes for Agents

- Main-view and editor bindings share the declarative `keymap!` macro in
  `action/keymap.rs`. Entries use Rust patterns and optional modifier clauses:
  `has(ALT + CONTROL)` allows extra modifiers, `exact(CONTROL)` permits only the
  named combination, and `without(CONTROL + ALT)` rejects either modifier.
  Unqualified entries ignore modifiers. Bindings match from top to bottom;
  the final wildcard returns `Action::QuickSearchClear` in the main view or
  `EditorCommand::Ignore` in the editor.
- Add non-modal bindings to `Action` and `action::resolve`. Editor bindings
  belong to the declarative keymap in `action/editor.rs`, returning a typed
  `EditorCommand`; `popup/editor.rs` interprets it after confirmation/search
  handling. The resolver performs no state changes or clipboard access.
- Add other modal behavior to the relevant `Overlay` arm in
  `popup::process_popup_input`.
- Prefer adding new variants to the `Overlay` enum instead of new boolean flags.
- Keep the modal-first boundary in `application/dispatch.rs`; do not bypass it for global
  shortcuts.

See `src/application/mod.rs` (event loop), `src/application/dispatch.rs`
(effect and input dispatch), `src/action.rs` (main-view key map), and
`src/popup.rs` with its focused children (modal input).
