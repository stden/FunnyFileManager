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
| F3        | View                           | ✅         | Internal viewer: Far's unwrapped text mode with its status line; no hex/dump, no key bar. Decodes as F4 does (`TextEncoding::detect`: a BOM's UTF-8/UTF-16, then UTF-8, else Windows-1251) and names that encoding; no F8/Shift+F8 code-page switch yet |
| F4        | Edit                           | ✅         | Built-in editor (Far style); Alt+F4 = external editor |
| Shift+F4  | Create new file                | ✅         |       |
| F5        | Copy                           | ✅         |       |
| F6        | Move / Rename                  | ✅         |       |
| Alt+F6    | Link                           | ✅         | Name prompt titled Link: link name (the other panel's directory + the entry's name by default; a bare name is relative to the other panel), Link type symbolic ⇄ hard (hard for files only — the arrows never stop on it for a folder), Target = the entry under the cursor, absolute. No directory junction: a Windows-only kind. Never overwrites an existing name |
| F7        | Mkdir                          | ✅         |       |
| F8        | Delete                         | ✅         | With confirmation dialog |
| Alt+Del   | Wipe                           | ✅         | Confirmation titled Wipe; each file is zero-filled, truncated, renamed to a temporary name and removed; directories recurse; symlinks are unlinked, never followed; pipes, sockets and devices are never opened, only unlinked; a file with other hard links is only unlinked and the status bar says so. Far's `System.WipeSymbol` filler is not offered (always zeros) |
| F9        | Menu bar                       | ✅         |       |
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

## 🧭 Panel Kinds (Left / Right menu)

| Key     | Far Manager Action      | FFM Status | Notes |
|---------|-------------------------|------------|-------|
| Ctrl+Q  | Toggle quick view panel | ✅         | As in Far, the *passive* panel becomes the preview and the cursor stays on the file list; pressed inside the preview it turns that panel back into a file list. A file shows its first screen through the viewer's decoder and highlighter (the encoding label, size and percent in the info row; binary, oversized and non-regular files are declined with a one-line reason); a folder shows its first readable configured description (`README.md`, then `README.txt` by default) while InfoPanel settings is on, otherwise `crate::stats` totals (size, files, folders) — a folder that cannot be read says `Cannot read: <reason>`, and folders inside it that could not be read mark the totals as lower bounds, as F3's Information box does. Unusable descriptions are skipped; changing the description settings updates the open preview. The preview follows the cursor of a file or tree panel on every event. With the preview panel active, Up/Down, PgUp/PgDn, Home/End, Left/Right and Ctrl+Left/Ctrl+Right scroll it like the viewer; every other key falls through. The key and the Left/Right menu row share one path: `MenuCommand::QuickView` → `ops::quickview::toggle`. |

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

## 🪟 Overlay / Dialog Keys

- Esc — universally closes current overlay (consistent with Far)
- Y/N in delete confirmation — implemented
- Viewer keys — implemented as in Far: Up/Down one line (the last line stops
  on the bottom row), PgUp/PgDn one screen, Home/End first/last screen,
  Left/Right one column, Ctrl+Left/Ctrl+Right twenty columns; Esc/Enter/F3/F10
  close
- F2 (bookmark overlay) and 0-9 / Ctrl+0-9 slot keys — ❌ never implemented; use `:bookmark set|go <0-9>`

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

## ↔️ Known Differences (Intentional)

- F4 opens the built-in editor, with syntax highlighting and optional language servers. Alt+F4 delegates to the external editor.
- Some advanced Far plugins and menu systems have no equivalent.
- F9 → Commands → `Install MCP server` (hotkey `e`) is an FFM extra with no Far counterpart: it registers `ffm-mcp` with AI clients. See [session tools](../../session-tools.md#mcp-server-setup).
- Drive letters on Unix are simulated as `/`.
- With the `s3` feature, credential-file profiles appear as extra drive-menu rows. Selecting one does not open a bucket. See [S3](../../tutorials/s3.md).

## 🧑‍💻 Far Manager Reference

Upstream documentation can be fetched on demand (see `docs/far-manager/README.md`).

This document is the living parity checklist for the project.
