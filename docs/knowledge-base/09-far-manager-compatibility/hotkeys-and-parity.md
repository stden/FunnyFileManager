<!-- Purpose: Tracks Far Manager hotkey parity and behavior expectations. -->

# ⌨️ Far Manager Hotkey Compatibility

This document tracks how closely FunnyFileManager matches classic Far Manager keyboard behavior.

## 🗝️ Legend
- ✅ Fully implemented
- ⚠️ Partially implemented or different behavior
- ❌ Not implemented (by design or future work)

## 🗂️ Navigation

| Key          | Far Manager Action          | FFM Status     | Notes |
|--------------|-----------------------------|----------------|-------|
| Up / Down    | Move cursor                 | ✅             |       |
| Home / End   | First / Last entry          | ✅             |       |
| Enter        | Enter directory / execute   | ✅             | A program keeps the real terminal; its output stays on screen under `[exit status: N] Press any key to return to FFM` until a key, and Ctrl+O's transcript records the run and its exit status |
| Tab          | Switch panels               | ✅             | Ignored while a panel is hidden (Ctrl+F1 / Ctrl+F2), as in Far |
| Alt+Letter   | Quick search                | ✅             |       |
| Ctrl+\       | Go to root                  | ✅             | Also on terminals that send it as Ctrl+4's byte; see Terminal Key Encoding below |
| Backspace    | Go to parent                | ❌             | Not implemented |

## 📋 File Operations (F-keys)

| Key       | Far Action                     | FFM Status | Notes |
|-----------|--------------------------------|------------|-------|
| F1        | Help                           | ✅         |       |
| Shift+F1  | Add to archive                 | ✅         | Dialog: name (default `<stem>.tar.gz` in the other panel), format radio tar.gz / tar / zip (zip only when `zip` is on PATH), "Delete files after archiving"; runs `tar`/`zip` without a shell, echoes the command in the console, never overwrites |
| Shift+F2  | Extract files                  | ✅         | Dialog: every marked `.tar`/`.tar.gz`/`.tgz`/`.tar.xz`/`.tar.bz2`/`.zip`, destination offered as the other panel (a relative path is taken from the active one, created if missing); members are listed and checked first — paths outside the destination and links are refused; an archive that would replace an existing file or write through a link already in the destination is refused and extracts nothing (Far's Overwrite / Skip / Rename prompt is not offered yet); marks are spent only on full success |
| Shift+F3  | Archive commands               | ✅         | Menu: Test archive, List contents (viewer), Extract here, Extract to other panel, Add selected files to this archive (`.tar`/`.zip`); `tar` reads the tar family, `unzip` reads zip |
| F2        | User menu                      | ✅         | Lists `user-menu.txt` (`Label<TAB>command`) and an Edit row that opens that file |
| F3        | View                           | ✅         | Internal viewer: Far's unwrapped text mode with its status line; no hex/dump, no key bar. Decodes as F4 does (`TextEncoding::detect`: a BOM's UTF-8/UTF-16, then UTF-8, else Windows-1251) and names that encoding; no F8/Shift+F8 code-page switch yet. The whole file is read into memory, so a file over 10 MB is refused with its size and the limit; there is no streaming mode yet |
| F4        | Edit                           | ✅         | Built-in editor (Far style); Alt+F4 = external editor |
| Shift+F4  | Create new file                | ✅         |       |
| F5        | Copy                           | ✅         | A taken name asks "Destination already exists": sizes and times of both, Overwrite / Skip / Keep both / Cancel, Remember choice |
| F6        | Move / Rename                  | ✅         | Asks about a taken name as F5 does |
| Alt+F6    | Link                           | ✅         | Name prompt titled Link: link name (the other panel's directory + the entry's name by default; a bare name is relative to the other panel), Link type symbolic ⇄ hard (hard for files only — the arrows never stop on it for a folder), Target = the entry under the cursor, absolute. No directory junction: a Windows-only kind. Never overwrites an existing name |
| F7        | Mkdir                          | ✅         |       |
| F8        | Delete                         | ✅         | With confirmation dialog |
| Alt+Del   | Wipe                           | ✅         | Confirmation titled Wipe; each file is zero-filled, truncated, renamed to a temporary name and removed; directories recurse; symlinks are unlinked, never followed; pipes, sockets and devices are never opened, only unlinked; a file with other hard links is only unlinked and the status bar says so. Far's `System.WipeSymbol` filler is not offered (always zeros) |
| F9        | Menu bar                       | ✅         |       |
| Shift+F9  | Save setup                     | ✅         | Writes the Options entries under `options` in `settings.yaml`; every other key keeps what the file has. A file that does not parse is not rewritten (`Not saved: …`) |
| Alt+F9    | Video mode (25 / 50 rows)      | ⚠️         | Asks the terminal to resize (`CSI 8 t`); many terminals, tmux among them, ignore it. The status bar says the height asked for and the height the terminal has |
| F10       | Quit                           | ✅         | Also Ctrl+C and the `quit` command |
| F11       | Plugin commands                | ✅         | Far's list: MCP status (the server, then Claude Code, Codex, Junie and Kiro at both scopes), NetBox: stored sessions, NetBox: import from `.netbox` file… |
| F12       | Switch screens                 | ✅         | The same list as F9 Screens: file panels or the console transcript. Ctrl+O toggles the console directly |
| Alt+F1/F2 | Drive selection                | ✅         | Under the rule, with digit keys from 0: every S3 profile, then `NetBox` (the stored-sessions panel on that side) and `Process list` (the Task list, as Ctrl+W). Del, Shift+Del, F3, F4, F9 and Ctrl+R of Far's dialog are not implemented |
| Ctrl+O    | Toggle console (hide panels)   | ✅         |       |
| Ctrl+U    | Swap panels                    | ✅         | Ignored while a panel is hidden (Ctrl+F1 / Ctrl+F2), as in Far; the status line says so |
| Ctrl+Z    | Describe files                 | ✅         | One prompt per marked entry (else the cursor), opening on the current text; writes Far's `name text` line (name quoted when it holds a space) to the first of `Settings.descriptions.names`, an empty text removes it, and the panel is re-listed |
| Ctrl+G    | Apply command                  | ✅         | Far file/path/selection substitutions, native short names, active/passive/left/right panel prefixes and `!@...!` / `!$...!` list modifiers; runs in panel order and unmarks processed entries (Ctrl+M restores). As in Far, names are inserted unescaped and the shell parses the line, so a crafted file name can run commands; the dialog warns, see [names are not escaped](../07-core-features/apply-command.md#names-are-not-escaped). See [syntax and scope](../07-core-features/apply-command.md); interactive parameter dialogs are not implemented |
| Ctrl+F1 / Ctrl+F2 | Hide / show left / right panel | ✅ | The visible panel takes the whole width (Far leaves the console in the freed half); hiding the active panel makes the other active; Tab, Left / Right and the F9 bar never land on a hidden panel; the last panel cannot be hidden — the status line says so (Ctrl+O hides both) |
| Ctrl+T    | Tree panel                     | ✅         | The active panel becomes the folder tree, rooted at the filesystem root and opened on the panel's own folder; Enter turns it back into a file panel there. No `tree3.far` cache: folders are listed as they are opened |

## 🌳 Tree Panel Keys

| Key            | Far Action                          | FFM Status | Notes |
|----------------|-------------------------------------|------------|-------|
| Up / Down / Home / End / PgUp / PgDn | Move cursor          | ✅         | With Options → Tree settings → Auto change folder (default on) the other panel follows the cursor |
| Right          | Open the folder's subfolders        | ✅         | On an open folder, steps to its first subfolder |
| Left           | Close the folder                    | ✅         | On a closed folder, climbs to its parent |
| Enter          | Go to the folder                    | ✅         | The panel is a file panel again, at the folder under the cursor |
| Gray + / Gray - | Next / previous folder on the same level | ✅    | Plain `+` / `-`; terminals cannot tell the numeric pad's keys from the row's |
| Alt+Letter     | Fast find                           | ✅         | Plain letters search too: a tree row is nothing a typed command could run. `Ctrl+Enter` selects the next match, Backspace shortens, Esc ends the search |
| Ctrl+R         | Re-read the tree                    | ✅         | Lists the open folders again |
| F5 – F8        | Folder operations from the tree     | ⚠️         | Refused with a status line: press Enter to open the folder, then use them on the file panel |

## 🎯 Marking

| Key          | Far Action              | FFM Status | Notes |
|--------------|-------------------------|------------|-------|
| Insert / Space | Toggle mark + move next | ✅         |       |
| Ctrl+A       | File attributes         | ✅         | Far's dialog for the marked entries (or the cursor): three-state permission boxes, modification/access times, Process subfolders; only the boxes and time fields the user changed are applied, and Process subfolders never follows a link |

## 🔃 Sort Modes

| Key        | Far Action                 | FFM Status | Notes |
|------------|----------------------------|------------|-------|
| Ctrl+F12   | Sort modes menu            | ✅         | Far's drop-down on the active panel: the nine modes with the mode in force ticked, then under a rule "Reverse order", ticked while the panel runs the other way. Opens on the current mode, so Enter alone reverses it; a row's letter, its `Ctrl+F` key or a click chooses; `+` `-` `*` set direct, reversed, flipped; Esc / F10 cancel |
| Ctrl+F3    | Sort by name               | ✅         | Pressed again while in force, reverses the order — as every direct key below |
| Ctrl+F4    | Sort by extension          | ✅         |       |
| Ctrl+F5    | Sort by modification time  | ✅         |       |
| Ctrl+F6    | Sort by size               | ✅         |       |
| Ctrl+F7    | Unsorted                   | ✅         | The directory's own order |
| Ctrl+F8    | Sort by creation time      | ✅         |       |
| Ctrl+F9    | Sort by access time        | ✅         |       |
| Ctrl+F10   | Sort by description        | ✅         |       |
| Ctrl+F11   | Sort by file owner         | ✅         |       |
| Shift+F11  | Use group sorting          | ❌         | Far's sort groups have no FFM equivalent |
| Shift+F12  | Show selected files first  | ❌         | Not implemented |
| F4 (in menu) | Sort criteria            | ❌         | Far's per-mode tie-break editor; FFM breaks ties by name |

## 💻 Command Line

| Command              | Supported | Notes |
|----------------------|-----------|-------|
| `cd`, `pwd`, `sort`  | ✅        |       |
| `bookmark set/go`    | ✅        | Uses `BookmarkSlot` type; the only way to reach bookmarks — F2 opens Commands and there is no bookmark overlay |
| `mark all`, `unmark` | ✅        |       |
| `mcp`, `winmsg`, etc.| ✅        |       |
| `sessions`           | ✅        | NetBox's stored-sessions panel; `sessions import netbox [path]` imports a `.netbox` export (the file under a file panel's cursor when no path is given) |
| Shell fallback       | ✅        | Any unknown command runs in shell |

### Keys that changed with exact matching

Command-line keys match their default chord exactly (keymap design spec §6.2): a
modifier nobody binds is no longer ignored. These keys acted and now do
nothing:

- Enter, Esc, Up, Down and Backspace with Shift, Ctrl or Alt, and Tab with Ctrl
  or Alt: Shift+Enter and Alt+Enter no longer run the line; Shift+Up and
  Ctrl+Down no longer step the history; Alt+Tab no longer completes.
- Del with Ctrl or Alt: Ctrl+Del and Alt+Del no longer delete a character.
  Shift+Del still cuts.
- Home and End with Ctrl or Alt: Ctrl+Home, Alt+Home, Ctrl+End and Alt+End no
  longer move the caret. Shift+Home and Shift+End still select; Ctrl+Shift+Home
  and Alt+Shift+End no longer do.
- Shift or Alt with a Ctrl+letter: Ctrl+Shift+C no longer copies or clears
  the line, and Ctrl+Alt+W no longer cuts the word before the caret.
- Alt+Shift with Left and Right, and Ctrl+Alt with Left and Right: Alt+Shift+Left
  no longer selects, and Ctrl+Alt+Right no longer moves a word. Shift+Left and
  Ctrl+Shift+Left still select.
- An extra modifier on the clipboard keys: Ctrl+Shift+Insert, Alt+Shift+Insert
  and Ctrl+Shift+Del no longer copy, paste or cut. Ctrl+Insert, Shift+Insert
  and Shift+Del still do.

The completion list and the Ctrl+R search lose the same keys: Shift+Enter no
longer accepts a candidate, and Ctrl+Shift+C no longer cancels a search.

Keys that now act: Ctrl+E and Alt+F take the grey suggestion when the caret is
at the end of the line, as Right, End, Ctrl+Right and Alt+Right already did.
Selection bindings such as Shift+Right, Shift+End and Ctrl+Shift+Right still
select instead of accepting it. Ctrl+Shift+E and Alt+Shift+F have no default
binding and do nothing.

## 🧭 Panel Kinds (Left / Right menu)

| Key     | Far Manager Action      | FFM Status | Notes |
|---------|-------------------------|------------|-------|
| Ctrl+Q  | Toggle quick view panel | ✅         | As in Far, the *passive* panel becomes the preview and the cursor stays on the file list; pressed inside the preview it turns that panel back into a file list. A file shows its first screen through the viewer's decoder and highlighter (the encoding label, size and percent in the info row; binary, oversized and non-regular files are declined with a one-line reason); a folder shows its first readable configured description (`README.md`, then `README.txt` by default) while InfoPanel settings is on, otherwise `crate::stats` totals (size, files, folders) — a folder that cannot be read says `Cannot read: <reason>`, and folders inside it that could not be read mark the totals as lower bounds, as F3's Information box does. Unusable descriptions are skipped; changing the description settings updates the open preview. The preview follows the cursor of a file or tree panel on every event. With the preview panel active, Up/Down, PgUp/PgDn, Home/End, Left/Right and Ctrl+Left/Ctrl+Right scroll it like the viewer; every other key falls through. The key and the Left/Right menu row share one path: `PanelCommand::QuickView` → `ops::quickview::toggle`. |

## 🌐 NetBox Panels

Far's NetBox plugin keeps SFTP/SCP/FTP connections as stored sessions. FFM
stores the sessions and imports NetBox's `.netbox` export; it has no network
transport yet, so connecting says so. See
`docs/knowledge-base/07-core-features/netbox-sessions.md`.

| Key (sessions panel) | Far NetBox Action        | FFM Status | Notes |
|----------------------|--------------------------|------------|-------|
| Enter on `..`        | Back to the file list    | ✅         |       |
| Enter on a session   | Connect                  | ⚠️         | This build has no SFTP/SCP/FTP transport: the status bar says so and the session stays stored |
| F3                   | Show the session         | ✅         | Its settings in the viewer; there is never a password to show |
| F4 / Shift+F4        | Edit / new session       | ✅         | Name, protocol, host, port, user, login (password / key file / agent), start directory. No password field: a password is asked for when connecting, never stored |
| F5                   | Copy session             | ✅         | Asks for the copy's name |
| F8                   | Delete session           | ✅         | Asks first |
| Alt+Letter           | Quick search             | ❌         | Refused with the other keys that act on files |
| Other file keys      | —                        | ✅         | F5–F8, marking, Alt+Del, Ctrl+A … are refused with a status line instead of acting on the file list hidden beneath; Tab, Alt+F1/F2, F9–F12 and the command line still work. F5/F6 from a file panel refuse a sessions or remote panel as the destination |

A remote panel (the seam the transport will fill; reached in this build only
through tests) moves with the list keys, Enter enters a directory, and `..`
at the host's root returns to the files.

The list keys take no modifier (exact matching, keymap spec §6.2): Shift+Up,
Shift+Down, Shift+Home, Shift+End, Shift+PgUp and Shift+PgDn no longer move
the sessions list or a remote host. With the panels' exact defaults, these
unbound chords fall through as `QuickSearchClear`; they do not act on the
hidden file list (keymap spec §6.2 and §14.5, step 5).

## 🪟 Overlay / Dialog Keys

- Esc — universally closes current overlay (consistent with Far)
- Text fields (rename, F5/F6 destination, new file, find file, archives,
  masks, prompts, command line) — ✅ Far's edit-control keys: Shift+arrows /
  Home / End select, Ctrl+A selects all, Ctrl+Insert / Ctrl+C copy,
  Shift+Delete / Ctrl+X cut, Shift+Insert / Ctrl+V paste. See
  `06-ui-and-interaction/keyboard-handling.md` for the fields not on it yet
- Y/N in delete confirmation — implemented
- Viewer keys — implemented as in Far: Up/Down one line (the last line stops
  on the bottom row), PgUp/PgDn one screen, Home/End first/last screen,
  Left/Right one column, Ctrl+Left/Ctrl+Right twenty columns; Esc/Enter/F3/F10
  close
- F2 (bookmark overlay) and 0-9 / Ctrl+0-9 slot keys — ❌ never implemented; use `:bookmark set|go <0-9>`
- Shift+F9 (Save setup) writes only the Options entries under `options` and
  refuses a file that does not parse, so it never erases the language servers
  or the typed pages

## 🧰 Language servers dialog

F9 → Commands → Language servers (LSP), or `lsp` on the command line. While it
is open every key goes to it, so none of these clash with a panel or
command-line binding; each means what it already means in FFM or Far. Keys not
listed do nothing (F1, F6, F9, F11, F12, Shift+F9, Ctrl+Z, Alt+letters).
Shift+Space toggles Enabled in the list, as Space does: Shift is part of the
space character, so it is the same key. Super, Hyper and Meta are ignored, so
Super+F8 deletes a server as F8 does. Any other list or view key with Shift,
Ctrl or Alt held is another key and does nothing: Shift+F8 does not reset,
Alt+F7 does not add, Shift+F2 does not save.

Each mode's key bar and bottom-border legend name only the keys that act
there. The form and the exit question blank the key bar, since no F-key acts
in them; the form's legend reads `Enter OK  Esc Cancel  Tab Next  Ctrl+Space
List`, or with a suggestion list open what Up, Down, Enter and Esc do to it,
and the exit question's reads `Y Yes  N No  Esc Cancel`.

**List**

| Key | Action | Far / FFM precedent |
|-----|--------|---------------------|
| Up / Down | Previous / next server | Far lists |
| PgUp / PgDn | One list page | Far lists |
| Home / End | First / last server | Far lists |
| Enter, F4 | Edit the selected server | PATH editor Enter/F4; Far F4 = Edit |
| Space | Toggle Enabled | Far checkbox |
| Ins, F7 | Add a server | PATH editor Ins/F7 |
| Del, F8 | Catalog server: reset its command, arguments, extensions and on/off; added server: remove | PATH editor Del/F8 |
| F2 | Save to `settings.yaml` (only the changed fields) | Editor and PATH editor F2 |
| F3 | View every server | Far F3 = View |
| F5 | Check availability again | PATH editor F5 |
| Esc, F10 | Close; with changes, ask Save / Discard / Cancel | Editor exit, Far F10 |

A read-only dialog (no settings file, or one that does not load) answers
Space, Enter/F4, Ins/F7, Del/F8 and F2 by repeating its reason.

**View (F3)**: Up / Down one line, PgUp / PgDn one page, Home / End top /
bottom, F5 check again, F3 / Esc / F10 back to the list with the selection kept.

**Form**

| Key | Action | Far / FFM precedent |
|-----|--------|---------------------|
| Tab / Shift+Tab | Next / previous control, wrapping; an existing server's language is skipped | Far dialogs |
| Down / Up (no list open) | Next / previous control | Far dialogs |
| Characters, Backspace, Delete, Left, Right, Home, End | Edit the field | FFM prompts |
| Ctrl+Space, Ctrl+Down, Alt+Down | Open an active suggestion list | Editor Ctrl+Space; Far Ctrl+Down; Windows Alt+Down |
| Space | On Enabled: toggle. On a button: press it. In a field: a space | Far dialogs |
| Enter | OK, or Cancel when Cancel has the cursor | Far default button |
| Esc | Cancel the form; the list is unchanged | Far dialogs |

**Suggestion list.** A list opened by typing (Options → AutoComplete on) is
*passive*: nothing is highlighted and Enter still presses OK. One opened by
Ctrl+Space / Ctrl+Down / Alt+Down, or entered with Down / Up, is *active*.

| Key | Passive list | Active list |
|-----|--------------|-------------|
| Down / Up | Enter it on the first / last row | Move the highlight, wrapping |
| Enter | Close it, then the form's Enter (OK) | Put the highlighted suggestion in the field |
| Esc | Close the list; the form stays | The same |
| Tab / Shift+Tab | Close it and move the cursor | The same |
| Characters, Backspace, Delete | Edit and refilter; it closes when nothing matches | Edit and refilter from the first row |
| Left, Right, Home, End | Move the caret and close it | The same |

**Exit question**: the editor's keys — Left / Right / Tab move between the
buttons, Enter presses the focused one, Y = Yes (save, then close; a refused
save returns to the list with the reason), N = No (discard, nothing written),
C / Esc = Cancel.

**Mouse**

| Where | Wheel (sent as Up / Down) | Click |
|-------|---------------------------|-------|
| List | Previous / next server | Nothing: rows and the dialog's key bar are not hit-tested |
| View | One line | Nothing |
| Form | Previous / next control; with a list open, as Up / Down there | `{ OK }` / `[ Cancel ]` take the cursor, then Enter; a field takes the cursor and `Enabled` toggles, as in the Copy dialog; the frame drags the form; the suggestion list takes no click, so it never presses a button it covers |
| Exit question | Nothing | `Yes`, `No`, `Cancel` send `y`, `n`, Esc |

The form and the exit question open centred, however far the last one was
dragged.

**Terminal notes.** Ctrl+Space arrives as NUL on xterm-family terminals, which
crossterm reports as Ctrl+Space; some input methods (IBus) take it first. On
macOS, Ctrl+Space is "Select the previous input source" and Ctrl+Down is
Mission Control's "Application windows" by default, so neither may reach the
terminal; Alt+Down (Option+Down) does wherever the terminal sends Option as
Alt, and with AutoComplete on, typing opens the list by itself.

## ✏️ Editor Keys (Exact Matching)

The editor's keys are rows of the `editor` table (`src/keymap/tables/editor.rs`).
A row runs only for the chord with exactly its modifiers, as the design spec's
§6.2 says (`docs/superpowers/specs/2026-09-24-ffm-yml-keymap-design.md`). Before
the switch an editor key still ran its command with an extra modifier that
nothing bound, so the keys below stopped acting. They are the 106 chords
that were `lenient` in `tests/keymap_golden/editor.tsv`; each now reads
`fallback` there and runs nothing. The bare keys and Shift alone on the
caret moves are unchanged.

| Keys | Was | Now |
|------|-----|-----|
| Left, Right, Up, Down, Home, End, PgUp, PgDn with Alt or Ctrl, with or without Shift: Ctrl+Left, Ctrl+Right, Ctrl+Up, Alt+Shift+Home | the caret move; with Shift, the selection | nothing |
| Enter, Backspace, Del, Esc, F2, F8, F10 with Shift, Alt or Ctrl: Alt+Enter, Shift+Enter, Ctrl+Del, Ctrl+Backspace, Shift+F2, Alt+F8 | new line, delete, close, save, choose the encoding | nothing |
| F7 with Alt or Ctrl, with or without Shift: Alt+F7, Ctrl+F7 | search; with Shift, repeat the search | nothing; Shift+F7 still repeats the search |
| Tab with Alt or Ctrl: Alt+Tab, Ctrl+Tab | indent | nothing |

**Capitals under Ctrl (Unix only).** On Unix a capital letter under Ctrl, or
under Ctrl and Alt, is the Shift chord, so Ctrl+Shift+C (kitty sends it as a
capital C under Ctrl) no longer copies, Ctrl+Shift+F no longer types the file
name, and Ctrl+Alt+Shift+L no longer formats the document. The Windows console
sends the same capital for `CapsLock`, which keeps Ctrl+C, Ctrl+F and
Ctrl+Alt+L as they were. The lowercase keys work on every platform.

Nobody can give these keys back yet: `ffm.yml` is read by a later step of the
keymap design (spec §10 step 8), so until then the defaults are the only keys.

## 🛤️ PATH editor dialog

F9 → Commands → Edit PATH (current user or all users). While its list is
browsed, a key acts only with exactly the modifiers of its own chord, and keys
not listed do nothing. The plain keys are unchanged, but Shift, Alt or Ctrl
added to Up, Down, Home, End, Ins, F7, Enter, F4, Del, F8, F2, F5 or Esc does
nothing: Shift+Up does not move the cursor, Alt+Home does not go to the first
entry, Ctrl+Home does not either, Shift+Del does not drop an entry, Shift+F2
does not save and Alt+Enter does not edit. Ctrl+Up and Ctrl+Down still move the
entry, but not with Shift or Alt added: Ctrl+Shift+Up and Ctrl+Alt+Down do
nothing. The entry field and the two questions keep their own keys.

## 🔐 KeePass vault

Enter or F3 on a `.kdbx` file opens the vault. While it is open every key goes
to it, so none of these clash with a panel or command-line binding. A character
is typed into the password while the vault is locked, and into its filter once
it is unlocked. The other keys do nothing (F1, F3, F9, F10, Tab, Ins, Del, Left,
Right).

A vault key with Shift, Ctrl or Alt held is another key and does nothing too;
Shift only changes the case of a typed character. So the Shift, Ctrl and Alt
variants of F2, F4, F6, F8, Enter, Esc, Backspace and the six list moves (Up,
Down, PgUp, PgDn, Home, End) do nothing, and so do Ctrl+R, Ctrl+U and Ctrl+Z
with Shift or Alt: Shift+F8 does not delete, Alt+Enter does not unlock, Ctrl+Up
does not move. On kitty-protocol terminals Ctrl+Shift+L does not lock either,
because the terminal sends it as an upper-case L with Ctrl.

## 🧭 Left / Right Menu

| Key    | Far Action      | FFM Status | Notes |
|--------|-----------------|------------|-------|
| Ctrl+N | Show long names | ✅         | Far reads the 8.3 name from FAT/NTFS; FFM computes it Windows's way on every platform (`ShortNames` in `src/models/shortname.rs`): spaces and leading dots dropped, uppercased, `+,;=[]` and non-ASCII become `_`, a name that then fits 8+3 is kept, anything longer becomes `LONGNA~1.EXT` with the number unique within the listing. The panel rows show the short form; the info row, marks and every operation keep the real name. |

## 🖥️ Terminal Key Encoding

Terminals send keys as bytes, and without help some combinations share a
byte. At startup FFM asks the terminal whether it speaks the kitty keyboard
protocol (`CSI ? u`). Where it does — kitty, foot, WezTerm with
`enable_kitty_keyboard`, Alacritty, Ghostty — FFM pushes
it (`CSI > 5 u`: disambiguated escape codes and alternate keys) and every
combination below arrives as itself. The flags are popped before the
alternate screen is left: on exit, before an external program or editor
runs (pushed again afterwards), and from the panic hook. The Windows console
reports key codes, not bytes, and is not affected.

On a terminal without the protocol — xterm, tmux, GNOME Terminal / VTE,
Konsole, the Linux console — these limits remain:

| Key | What the terminal sends | What FFM does | Use instead |
|-----|-------------------------|---------------|-------------|
| Ctrl+\ and Ctrl+4 | One byte, 0x1C, which crossterm reports as Ctrl+4 | Go to root: `terminal::decode_input` rewrites it before any keymap sees it, so Ctrl+\ never switches to Wide view | Wide view: F9 → Left / Right → Wide |
| Ctrl+] and Ctrl+5 | One byte, 0x1D, reported as Ctrl+5 | Panels: Detailed view (Ctrl+5). Editor: Go to definition (Ctrl+]), which has no Ctrl+5 of its own | — |
| Ctrl+I | Tab's byte (0x09) | Tab: switch panels | F9 → Commands → File panel filter |
| Ctrl+M | Enter's byte (0x0D) | Enter | F9 → Files → Restore selection |
| Ctrl+F3, Shift+F3, Alt+F3 | xterm's `CSI 1;5R` / `1;2R` / `1;3R`, which crossterm 0.29 reads as a cursor-position report | Nothing: the key never reaches FFM | Ctrl+F12 then `N` (sort by name); F9 → Files → Archive commands |
| Ctrl+1 … Ctrl+0 | xterm and VTE: the bare digit (1, 9, 0), NUL (2), Esc (3), DEL (8), or the bytes above (4 – 7) | Only Ctrl+5 … Ctrl+7 reach their view mode | F9 → Left / Right → the view mode rows |

Non-Latin layouts: with the protocol pushed, a Ctrl+letter pressed in a
Cyrillic (or other non-Latin) layout arrives as that letter — for example
Ctrl+С rather than Ctrl+C — because crossterm 0.29 does not read the
protocol's base-layout key. Switch to the Latin layout for Ctrl shortcuts on
those terminals.

## WSL manager keys (exact matching)

A screen that reads its keys from the keymap (`App.keymap`) runs a command
only for a key held with exactly the modifiers of its binding: a modifier
more is another key. The chords below ran the bare key's command before and
do nothing now. Each is a `fallback` row of the screen's table in
`tests/keymap_golden/`.

| Screen | Chords that no longer answer | What they did |
|--------|------------------------------|---------------|
| WSL manager (`wsl_manager`) | Up, Down, Home, End and Esc held with Shift, Alt or Ctrl in any combination; Shift+Enter (36 chords) | Moved the selection, closed the manager, opened the distribution. The manager still consumes them: none reaches the panels behind it. F2, F5 and F6 already answered only without a modifier. |

## ↔️ Known Differences (Intentional)

- F4 opens the built-in editor, with syntax highlighting and optional language servers. Alt+F4 delegates to the external editor.
- Some advanced Far plugins and menu systems have no equivalent.
- F9 → Commands → `Install MCP server` (hotkey `e`) is an FFM extra with no Far counterpart: it registers `ffm-mcp` with AI clients. See [session tools](../../session-tools.md#mcp-server-setup).
- F9 → Commands → `Space analysis` is an FFM extra with no Far counterpart and no hotkey, since every letter of its label is another Commands row's hotkey. It only suggests what to delete; see [main menu](../07-core-features/main-menu.md).
- F9 → Commands → `Things catalog` (no hotkey) is an FFM extra with no Far counterpart: it imports the machines of the Things catalog as SFTP sessions. See [Things catalog](../07-core-features/things-catalog.md).
- Drive letters on Unix are simulated as `/`.
- With the `s3` feature, credential-file profiles appear as extra drive-menu rows. Selecting one does not open a bucket. See [S3](../../tutorials/s3.md).

## 🔁 Leniency List (Exact Matching)

### File panels

Each modifier is exact: `Shift+F5` is not `F5`, and a chord nothing binds is another key. Before the keymap switch, these chords ran the bare key's command. Now a chord nothing binds follows the panels' fallback: Alt with a character starts a quick search, any other printable key opens the command line, and every other key clears the quick search. The table lists every chord that changed, from the regenerated `tests/keymap_golden/panels.tsv`. File remapping is planned for migration step 8; once it lands, an explicit binding such as `copy: ['F5', 'Shift+F5']` in `ffm.yml` can retain a chord (spec §6.2).

| Used to run | Chords that no longer run it |
|---|---|
| `*` invert selection | `Ctrl+*`, `Ctrl+Shift+*` |
| `+` select group | `Ctrl++`, `Ctrl+Shift++` |
| `-` unselect group | `Ctrl+-`, `Ctrl+Shift+-` |
| Ctrl+1 … Ctrl+0 view modes | `Ctrl+Shift+0`, `Ctrl+Alt+0`, `Ctrl+Alt+Shift+0`, `Ctrl+Shift+1`, `Ctrl+Alt+1`, `Ctrl+Alt+Shift+1`, `Ctrl+Shift+2`, `Ctrl+Alt+2`, `Ctrl+Alt+Shift+2`, `Ctrl+Shift+3`, `Ctrl+Alt+3`, `Ctrl+Alt+Shift+3`, `Ctrl+Shift+4`, `Ctrl+Alt+4`, `Ctrl+Alt+Shift+4`, `Ctrl+Shift+5`, `Ctrl+Alt+5`, `Ctrl+Alt+Shift+5`, `Ctrl+Shift+6`, `Ctrl+Alt+6`, `Ctrl+Alt+Shift+6`, `Ctrl+Shift+7`, `Ctrl+Alt+7`, `Ctrl+Alt+Shift+7`, `Ctrl+Shift+8`, `Ctrl+Alt+8`, `Ctrl+Alt+Shift+8`, `Ctrl+Shift+9`, `Ctrl+Alt+9`, `Ctrl+Alt+Shift+9` |
| : | `Alt+:` (now quick search) |
| Ctrl+\ go to root | `Ctrl+Shift+\`, `Ctrl+Alt+\`, `Ctrl+Alt+Shift+\` |
| Ctrl+A attributes | `Ctrl+Shift+A`, `Ctrl+Alt+A`, `Ctrl+Alt+Shift+A` |
| Ctrl+C quit | `Ctrl+Shift+C`, `Ctrl+Alt+C`, `Ctrl+Alt+Shift+C` |
| Ctrl+G apply command | `Ctrl+Shift+G`, `Ctrl+Alt+G`, `Ctrl+Alt+Shift+G` |
| Ctrl+I file panel filter | `Ctrl+Shift+I`, `Ctrl+Alt+I`, `Ctrl+Alt+Shift+I` |
| Ctrl+L information | `Ctrl+Shift+L`, `Ctrl+Alt+L`, `Ctrl+Alt+Shift+L` |
| Ctrl+M restore selection | `Ctrl+Shift+M`, `Ctrl+Alt+M`, `Ctrl+Alt+Shift+M` |
| Ctrl+N long names | `Ctrl+Shift+N`, `Ctrl+Alt+N`, `Ctrl+Alt+Shift+N` |
| Ctrl+O console | `Ctrl+Shift+O`, `Ctrl+Alt+O`, `Ctrl+Alt+Shift+O` |
| Ctrl+Q quick view | `Ctrl+Shift+Q`, `Ctrl+Alt+Q`, `Ctrl+Alt+Shift+Q` |
| Ctrl+R refresh | `Ctrl+Shift+R`, `Ctrl+Alt+R`, `Ctrl+Alt+Shift+R` |
| Ctrl+U swap panels | `Ctrl+Shift+U`, `Ctrl+Alt+U`, `Ctrl+Alt+Shift+U` |
| Ctrl+W task list | `Ctrl+Shift+W` |
| Ctrl+Alt+W WSL manager | `Ctrl+Alt+Shift+W` |
| Ctrl+Z describe | `Ctrl+Shift+Z`, `Ctrl+Alt+Z`, `Ctrl+Alt+Shift+Z` |
| Alt+F1 drive menu, left | `Alt+Shift+F1`, `Ctrl+Alt+F1`, `Ctrl+Alt+Shift+F1` |
| Alt+F2 drive menu, right | `Alt+Shift+F2`, `Ctrl+Alt+F2`, `Ctrl+Alt+Shift+F2` |
| Shift+F1 add to archive | `Ctrl+Shift+F1` |
| Shift+F2 extract | `Ctrl+Shift+F2` |
| F3 view | `Alt+F3`, `Alt+Shift+F3`, `Ctrl+Alt+F3`, `Ctrl+Alt+Shift+F3` |
| Shift+F3 archive commands | `Ctrl+Shift+F3` |
| Alt+F4 external editor | `Alt+Shift+F4`, `Ctrl+Alt+F4`, `Ctrl+Alt+Shift+F4` |
| Shift+F4 new file | `Ctrl+Shift+F4` |
| F5 copy | `Shift+F5`, `Alt+F5`, `Alt+Shift+F5`, `Ctrl+Shift+F5`, `Ctrl+Alt+F5`, `Ctrl+Alt+Shift+F5` |
| Alt+F6 link | `Alt+Shift+F6`, `Ctrl+Alt+F6`, `Ctrl+Alt+Shift+F6` |
| Shift+F6 rename | `Ctrl+Shift+F6` |
| F7 new folder | `Shift+F7`, `Ctrl+Shift+F7` |
| Alt+F7 find file | `Alt+Shift+F7`, `Ctrl+Alt+F7`, `Ctrl+Alt+Shift+F7` |
| F8 delete | `Shift+F8`, `Ctrl+Shift+F8` |
| Alt+F8 command history | `Alt+Shift+F8`, `Ctrl+Alt+F8`, `Ctrl+Alt+Shift+F8` |
| Alt+F9 video mode | `Alt+Shift+F9`, `Ctrl+Alt+F9`, `Ctrl+Alt+Shift+F9` |
| F9 menu bar | `Ctrl+Shift+F9` |
| F10 quit | `Shift+F10`, `Ctrl+Shift+F10` |
| Alt+F10 find folder | `Alt+Shift+F10`, `Ctrl+Alt+F10`, `Ctrl+Alt+Shift+F10` |
| F11 plugin commands | `Shift+F11`, `Ctrl+Shift+F11` |
| Alt+F11 view history | `Alt+Shift+F11`, `Ctrl+Alt+F11`, `Ctrl+Alt+Shift+F11` |
| F12 screens | `Shift+F12`, `Ctrl+Shift+F12` |
| Alt+F12 folder history | `Alt+Shift+F12`, `Ctrl+Alt+F12`, `Ctrl+Alt+Shift+F12` |
| Enter open | `Shift+Enter`, `Alt+Enter`, `Alt+Shift+Enter`, `Ctrl+Enter`, `Ctrl+Shift+Enter`, `Ctrl+Alt+Enter`, `Ctrl+Alt+Shift+Enter` |
| Tab switch panel | `Alt+Tab`, `Ctrl+Tab`, `Ctrl+Alt+Tab` |
| Backspace quick search | `Shift+Backspace`, `Alt+Backspace`, `Alt+Shift+Backspace`, `Ctrl+Backspace`, `Ctrl+Shift+Backspace`, `Ctrl+Alt+Backspace`, `Ctrl+Alt+Shift+Backspace` |
| Insert and Space mark | `Shift+Ins`, `Alt+Ins`, `Alt+Shift+Ins`, `Ctrl+Ins`, `Ctrl+Shift+Ins`, `Ctrl+Alt+Ins`, `Ctrl+Alt+Shift+Ins`, `Alt+Space` (now quick search), `Ctrl+Space`, `Ctrl+Shift+Space`, `Ctrl+Alt+Space`, `Ctrl+Alt+Shift+Space` |
| Alt+Delete wipe | `Alt+Shift+Del`, `Ctrl+Alt+Del`, `Ctrl+Alt+Shift+Del` |
| Home | `Shift+Home`, `Alt+Home`, `Alt+Shift+Home`, `Ctrl+Home`, `Ctrl+Shift+Home`, `Ctrl+Alt+Home`, `Ctrl+Alt+Shift+Home` |
| End | `Shift+End`, `Alt+End`, `Alt+Shift+End`, `Ctrl+End`, `Ctrl+Shift+End`, `Ctrl+Alt+End`, `Ctrl+Alt+Shift+End` |
| PgUp | `Shift+PgUp`, `Alt+PgUp`, `Alt+Shift+PgUp`, `Ctrl+PgUp`, `Ctrl+Shift+PgUp`, `Ctrl+Alt+PgUp`, `Ctrl+Alt+Shift+PgUp` |
| PgDn | `Shift+PgDn`, `Alt+PgDn`, `Alt+Shift+PgDn`, `Ctrl+PgDn`, `Ctrl+Shift+PgDn`, `Ctrl+Alt+PgDn`, `Ctrl+Alt+Shift+PgDn` |
| Up | `Shift+Up`, `Alt+Up`, `Alt+Shift+Up`, `Ctrl+Up`, `Ctrl+Shift+Up`, `Ctrl+Alt+Up`, `Ctrl+Alt+Shift+Up` |
| Down | `Shift+Down`, `Alt+Down`, `Alt+Shift+Down`, `Ctrl+Down`, `Ctrl+Shift+Down`, `Ctrl+Alt+Down`, `Ctrl+Alt+Shift+Down` |
| Left | `Shift+Left`, `Alt+Left`, `Alt+Shift+Left`, `Ctrl+Left`, `Ctrl+Shift+Left`, `Ctrl+Alt+Left`, `Ctrl+Alt+Shift+Left` |
| Right | `Shift+Right`, `Alt+Right`, `Alt+Shift+Right`, `Ctrl+Right`, `Ctrl+Shift+Right`, `Ctrl+Alt+Right`, `Ctrl+Alt+Shift+Right` |

Two chords now start a quick search instead of their old command: `Alt+:` searches for `:`, and `Alt+Space` for a space.

### Viewer and quick view

A key with a modifier that no binding names does nothing of its own: it no
longer acts as the bare key does. The chords a screen changes are pinned
in the golden tables under `tests/keymap_golden/`:

- **F3 viewer.** Esc, Enter, F3 and F10 with Shift, Alt or Ctrl added no
  longer close it. Up, Down, PgUp, PgDn, Home and End with a modifier added
  no longer scroll it, Ctrl+Up and Ctrl+Down included. Left and Right with
  Shift or Alt added no longer pan it; Ctrl+Left and Ctrl+Right still pan
  twenty columns. The viewer swallows these keys.
- **Ctrl+Q quick view.** The same scroll keys with a modifier added no longer
  scroll the preview. They reach the file panels' exact bindings. With the
  defaults, these modified scroll keys have no binding and clear the quick
  search instead of moving either panel. Ctrl+Left and Ctrl+Right still pan
  the preview twenty columns through the viewer's own bindings.

## 🧑‍💻 Far Manager Reference

Upstream documentation can be fetched on demand (see `docs/far-manager/README.md`).

This document is the living parity checklist for the project.
