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
| Enter        | Enter directory / execute   | ✅             |       |
| Tab          | Switch panels               | ✅             | Ignored while a panel is hidden (Ctrl+F1 / Ctrl+F2), as in Far |
| Alt+Letter   | Quick search                | ✅             |       |
| Ctrl+\       | Go to root                  | ✅             |       |
| Backspace    | Go to parent                | ❌             | Not implemented |

## 📋 File Operations (F-keys)

| Key       | Far Action                     | FFM Status | Notes |
|-----------|--------------------------------|------------|-------|
| F1        | Help                           | ✅         |       |
| Shift+F1  | Add to archive                 | ✅         | Dialog: name (default `<stem>.tar.gz` in the other panel), format radio tar.gz / tar / zip (zip only when `zip` is on PATH), "Delete files after archiving"; runs `tar`/`zip` without a shell, echoes the command in the console, never overwrites |
| Shift+F2  | Extract files                  | ✅         | Dialog: every marked `.tar`/`.tar.gz`/`.tgz`/`.tar.xz`/`.tar.bz2`/`.zip`, destination offered as the other panel (a relative path is taken from the active one, created if missing); members are listed and checked first — paths outside the destination and links are refused; marks are spent only on full success |
| Shift+F3  | Archive commands               | ✅         | Menu: Test archive, List contents (viewer), Extract here, Extract to other panel, Add selected files to this archive (`.tar`/`.zip`); `tar` reads the tar family, `unzip` reads zip |
| F2        | User menu                      | ✅         | Lists `user-menu.txt` (`Label<TAB>command`) and an Edit row that opens that file |
| F3        | View                           | ✅         | Internal viewer: Far's unwrapped text mode with its status line; no hex/dump, no key bar |
| F4        | Edit                           | ✅         | Built-in editor (Far style); Alt+F4 = external editor |
| Shift+F4  | Create new file                | ✅         |       |
| F5        | Copy                           | ✅         |       |
| F6        | Move / Rename                  | ✅         |       |
| Alt+F6    | Link                           | ✅         | Name prompt titled Link: link name (the other panel's directory + the entry's name by default; a bare name is relative to the other panel), Link type symbolic ⇄ hard (hard for files only — the arrows never stop on it for a folder), Target = the entry under the cursor, absolute. No directory junction: a Windows-only kind. Never overwrites an existing name |
| F7        | Mkdir                          | ✅         |       |
| F8        | Delete                         | ✅         | With confirmation dialog |
| Alt+Del   | Wipe                           | ✅         | Confirmation titled Wipe; each file is zero-filled, truncated, renamed to a temporary name and removed; directories recurse; symlinks are unlinked, never followed. Far's `System.WipeSymbol` filler is not offered (always zeros) |
| F9        | Menu bar                       | ✅         |       |
| F10       | Quit                           | ✅         | Also Ctrl+C and the `quit` command |
| F11       | Plugin commands                | ✅         | Opens MCP/plugin status |
| F12       | Switch screens                 | ✅         | The same list as F9 Screens: file panels or the console transcript. Ctrl+O toggles the console directly |
| Alt+F1/F2 | Drive selection                | ✅         |       |
| Ctrl+O    | Toggle console (hide panels)   | ✅         |       |
| Ctrl+U    | Swap panels                    | ✅         | Ignored while a panel is hidden (Ctrl+F1 / Ctrl+F2), as in Far; the status line says so |
| Ctrl+Z    | Describe files                 | ✅         | One prompt per marked entry (else the cursor), opening on the current text; writes Far's `name text` line (name quoted when it holds a space) to the first of `Settings.descriptions.names`, an empty text removes it, and the panel is re-listed |
| Ctrl+G    | Apply command                  | ✅         | Far file/path/selection substitutions, native short names, active/passive/left/right panel prefixes and `!@...!` / `!$...!` list modifiers; runs in panel order and unmarks processed entries (Ctrl+M restores). See [syntax and scope](../07-core-features/apply-command.md); interactive parameter dialogs are not implemented |
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
| Ctrl+A       | File attributes         | ✅         | Far's dialog for the marked entries (or the cursor): three-state permission boxes, modification/access times, Process subfolders |

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
| Shell fallback       | ✅        | Any unknown command runs in shell |

## 🧭 Panel Kinds (Left / Right menu)

| Key     | Far Manager Action      | FFM Status | Notes |
|---------|-------------------------|------------|-------|
| Ctrl+Q  | Toggle quick view panel | ✅         | As in Far, the *passive* panel becomes the preview and the cursor stays on the file list; pressed inside the preview it turns that panel back into a file list. A file shows its first screen through the viewer's decoder and highlighter (UTF-8/ANSI label, size and percent in the info row; binary, oversized and non-regular files are declined with a one-line reason); a folder shows its first readable configured description (`README.md`, then `README.txt` by default) while InfoPanel settings is on, otherwise `crate::stats` totals (size, files, folders). Unusable descriptions are skipped; changing the description settings updates the open preview. The preview follows the cursor of a file or tree panel on every event. With the preview panel active, Up/Down, PgUp/PgDn, Home/End, Left/Right and Ctrl+Left/Ctrl+Right scroll it like the viewer; every other key falls through. The key and the Left/Right menu row share one path: `MenuCommand::QuickView` → `ops::quickview::toggle`. |

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

## ↔️ Known Differences (Intentional)

- F4 opens the built-in editor, with syntax highlighting and optional language servers. Alt+F4 delegates to the external editor.
- Some advanced Far plugins and menu systems have no equivalent.
- Drive letters on Unix are simulated as `/`.
- With the `s3` feature, credential-file profiles appear as extra drive-menu rows. Selecting one does not open a bucket. See [S3](../../tutorials/s3.md).

## 🧑‍💻 Far Manager Reference

Upstream documentation can be fetched on demand (see `docs/far-manager/README.md`).

This document is the living parity checklist for the project.
