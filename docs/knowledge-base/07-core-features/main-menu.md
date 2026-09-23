# F9 main menu

Every non-separator F9 entry is executable. Menus use the existing keyboard
dispatcher or a typed `MenuCommand`; effects live in `src/ops/menu/`. The
behavior is tailored to FunnyFileManager rather than reproducing every Far
Manager dialog.

The Commands menu also provides action timing statistics, installed-package
updates, and separate user/system PATH editors. Their owners are
`performance` / `ops/performance`, `packages` / `ops/packages`, and
`path_env` / `ops/path_env`. See [session tools](../../session-tools.md) for
keyboard controls, persistence, and platform behavior.

## Panels

Left and Right configure their respective panel. Walking the bar across
their headings (arrows, a click on the heading, Esc) never changes the active
panel, so Files and Commands rows keep acting on the panel F9 was pressed in;
choosing a row of the Left or Right menu makes that panel active first,
because the keys behind those rows act on the active panel. Brief, Medium, Full and Wide
are one table layout (`src/ui/panel.rs`): Brief and Medium flow the names down
three and two columns, Full shows name/size/date/time and Wide name/size. Each
column is titled, ruled and closed on the divider, and the info row adds the
current entry's size, date and time that the view leaves out. In Brief and
Medium `Left`/`Right` move the cursor a column and the mouse maps clicks through
the same geometry (`ui::stripe_at`). Detailed adds byte sizes and timestamps; Alternative full shows names
and readable sizes. Description modes read the configured description sidecar;
Long descriptions reserves more width for that text. As in Far, the sidecar's
name ignores letter case: `descript.ion`, `Descript.ion` and `DESCRIPT.ION` are
one file for these views, the Descriptions sort and Describe (`src/description.rs`).
Owners and Links show cached filesystem ownership and link information,
refreshed during navigation; on Unix the owner is `user:group` by name, with
the number for an id that has no passwd or group entry.

`Ctrl+T` turns the active panel into a folder tree, rooted at the filesystem root and opened on that panel's folder. Enter turns it back into a file panel there. Folders are listed as they are opened. Quick view (Ctrl+Q) turns the
passive panel into a preview of the entry under the active cursor — a text file
decoded and highlighted as the viewer would, a folder's description or totals —
and follows the cursor on every event; from inside the preview it turns that panel back into a
file list. With Options → InfoPanel settings enabled, folders show the first
readable text file from Options → Folder description files (`README.md;README.txt`
by default). Missing, non-regular, binary, oversized or unreadable descriptions
are skipped; without a usable description, the panel shows folder totals.
Changes to either setting update an open preview immediately.
Sort modes changes actual file order. Panel On/Off hides one panel. Show long names (Ctrl+N) switches between
full names and Far's 8.3 form (`LONGNA~1.EXT`, computed by `ShortNames` in
`src/models/shortname.rs` and unique within the listing); it never renames
files or claims to retrieve native Windows short-path aliases. Re-read, drive selection and
information use the existing operations.

The sort chooser uses Far's compact vertical-menu appearance: a white double
frame and text on teal (`#008080`), a black selection bar, yellow mnemonic
letters, aligned shortcut hints, a shadow, and an arrow for the active order.
It opens beside the active panel. The rows are the nine sort modes — name, extension, modification time, size, unsorted, creation time, access time, description, and owner — plus Reverse order. `Ctrl+F3` through `Ctrl+F11` choose those modes directly, and pressing the key of the mode already in force reverses it. `+`, `-`, and `*` set direct, reversed, and flipped order. Arrow keys wrap, Enter applies, and a mouse click activates the clicked row.
The visual reference is Far's [default menu palette](https://github.com/FarGroup/FarManager/blob/master/far/palette.cpp)
and [sort-menu layout](https://github.com/FarGroup/FarManager/blob/master/far/filelist.cpp).

Other dialogs follow the same default Far palette. Input forms use a light-gray
surface (`#c0c0c0`), black text and cyan fields; destructive confirmations use
dark red (`#800000`). Result lists, histories, WSL and unlocked KeePass use
white text on teal with a black selection bar. Shared forms have an inset
double frame, a shadow and centered buttons. F9 prompts size to their captions,
wrap long explanations, and keep the input cursor visible when editing long
Unicode text. Their OK and Cancel (F10) buttons also work with the mouse;
clicking a result selects the row actually displayed, including after scrolling.

## Files and commands

Existing view/edit/copy/move/rename/delete/selection operations remain shared
with their shortcuts. Additional operations are:

| Item | Behavior |
|------|----------|
| Link | One selected file becomes a hard link; directories use symbolic links. Existing destinations are refused. Directory links may require host privileges. |
| Wipe | F8-style confirmation (Y/Enter wipes, N/Esc/F10 cancels); each file is overwritten with zeros, truncated, renamed to a temporary name and removed. Directories recurse; symlinks are unlinked, never followed; pipes, sockets and devices are never opened, only unlinked. A file with other hard links is only unlinked too (its data belongs to the other names), and the status bar says so. This does not guarantee erasure from SSDs, snapshots, or backups. |
| Add to archive | Far's dialog: name, format (tar.gz / tar / zip where `zip` is installed), "Delete files after archiving". Runs the host `tar` or `zip`; an existing archive is never overwritten. |
| Extract | Far's dialog for every marked archive, into the other panel by default (created if missing). Traversal paths, links and special archive members are refused. Each archive unpacks into a private folder first and moves in only if nothing in the destination would be replaced or written through a link; otherwise it is refused and nothing is extracted (existing folders are merged into). |
| Archive commands | Test archive, List contents, Extract here, Extract to other panel, Add selected files to this archive. |
| Attributes | Changes read-only/writable status; Unix also accepts octal permissions. |
| Apply command | Runs a shell template once per selected entry in panel order with [Far substitutions](apply-command.md): filenames and native short names, paths, panel prefixes and temporary selection lists. Every run is echoed into the Ctrl+O console; processed entries are unmarked and Ctrl+M restores the selection. Names are inserted unescaped, as in Far, so a crafted file name can run commands; the dialog warns ([details](apply-command.md#names-are-not-escaped)). |
| Describe | Updates the directory's existing description sidecar, whatever its letter case, while retaining other entries; creates the first configured name when there is none. |
| Find file/folder | Recursive name-mask search with configurable entry and depth limits. Folders that cannot be read are skipped; the status line says how many, and the matches found elsewhere are still listed. |
| Command history | Copies the selected command into the command line for review and execution. |
| File/folder history | Reopens a viewed file or previously visited directory. The file list is the persisted view history (`view_history` in the data directory, newest first), so files viewed or edited with F3, F4 or Alt+F4 in earlier sessions are listed too. |
| Video mode | Requests 25 rows from a terminal taller than 30, else 50, retaining width, then reports in the status bar the height the terminal has after up to 300 ms: `Video mode: 25 rows`, or `Video mode: asked for 25 rows, the terminal has 40` when the terminal ignores the request (tmux and most terminal tabs do). |
| Compare folders | Marks missing or different-sized files in both panels; this is not a recursive content comparison. |
| Edit user menu / associations | Opens configuration text in the built-in editor, where Tab types the TAB separator the format needs. |
| Folder shortcuts | Opens a saved bookmark; `:bookmark set N` creates one. |
| Panel filter | Filters files by mask; directories remain available for navigation. |
| Screens | Chooses file panels or console transcript. F9 from the console returns to panels and opens the menu. |
| Tasks / hotplug devices | Shows host process information or currently mounted drives. |
| Plugin commands | Far's F11 list: MCP status, NetBox's stored sessions, and the NetBox `.netbox` import (which asks for the file). |

The F9 row and its shortcut call the same `ops/menu` operation. Alt+F7 opens the Find file dialog, whose containing-text field narrows the name search. See [overlays](../../06-ui-and-interaction/overlays-and-modals.md).

User-menu files contain `Label<TAB>command` lines. Association files contain
`extension<TAB>command`; extension matching ignores case and `{}` expands to
the selected filename. Lines starting with `#` are comments. Commands selected
from the user menu are placed in the command line before execution.

## Options and persistence

Each Options entry opens a prompt explaining its supported setting. Enter
validates and applies it. Escape cancels when enabled; F10 always cancels.

| Options entry | Supported setting |
|---------------|-------------------|
| System / Tree | Maximum visited search entries / recursion depth |
| Panel / Interface | Hidden-file visibility / menu clock |
| Languages | English or the path of a JSON dictionary mapping menu row labels, including `&` mnemonic markers, to translated text; it must be a regular file of at most 1 MiB (a FIFO or device is refused unopened) |
| Plugins configuration | Additional command offered from the user menu |
| Plugins manager | Shell/plugin command timeout |
| Dialog / Menu | Escape dismissal / wrapping row navigation |
| Command line / AutoComplete | History length / completion and suggestions |
| InfoPanel | Folder descriptions in quick view |
| Groups of masks | Default semicolon-separated selection patterns |
| Confirmations | Delete confirmation |
| File panel modes | Default panel view |
| File / Folder descriptions | Sidecar filename / candidate folder-description filenames |
| Viewer / Editor | Tab display width / editor tab size (TAB display width and spaces inserted by Tab) |
| Code pages | Encoding for new files |
| Colors / Highlighting | Panel background / file-type colors |
| Save setup | Atomically persists settings for future sessions |

Invalid settings loads preserve the existing settings and both panels.

### The settings file

Settings are stored in `ffm/settings.yaml` in the configuration directory:
`%APPDATA%` on Windows; elsewhere `$XDG_CONFIG_HOME` when it is an absolute
path, otherwise `~/.config`. User-menu and association files live beside it.
Sessions constructed without a configuration location use `.ffm` under the
active directory for command files.

The file is one YAML mapping with two owners. Save setup writes the Options
entries under `options`, one key per entry named after it:

```yaml
options:
  Panel: off
  Editor: '8'
```

The typed settings (`lsp`, `system`, `panel`, `editor`, …) are top-level keys,
written at exit, and only when `system.auto_save_setup` is `true`. The entries
have a key of their own so that none sits beside a typed key that differs from
it only in case: the built-in editor reads `options.Editor`, not
`editor.tab_size`. Nothing else writes the file.

- **What a save writes.** Each writer remembers what it loaded (the Options
  entries: what they held at the last Save setup). A save writes only what
  changed since then and keeps every other value as the file holds it: the
  other writer's keys, keys a newer FFM wrote, and anything edited by hand
  while FFM runs, including in its own editor. A save with nothing to change
  leaves the file alone, comments included. The first save to a new file
  writes all of the writer's settings.
- **Hand edits.** YAML reads `Editor: 8` and `Panel: on` without quotes. Quote
  any other value with single quotes (`Masks: '!*.tmp'`,
  `Plugins: 'make # all'`): YAML changes some plain values without an error.
  In an Options entry an empty value, `~` and `null` are empty (a typed
  setting that expects text reports them instead); `#` after a space starts
  a comment, so `README.md #notes` is `README.md`; `0x10` is `16` and `+32`
  is `32`.
  Inside double quotes a backslash starts an escape (`"C:\temp"` holds a TAB),
  so write Windows paths in single quotes. A value that starts with `!` is a
  YAML tag, which the file refuses (see below); one that starts with `&` or `*`
  is an anchor or an alias. A save that writes the file rewrites all of it
  under a one-line header, so comments are then lost. An empty or
  comment-only file holds no settings.
- **A change made both by hand and in FFM** to the same value while FFM runs is
  not merged: Save setup stops with `Not saved: settings.yaml changed on disk:
  options.Editor was edited there and in this session`, and the exit auto-save
  logs the same line and writes nothing. Restart FFM to load the file as it is.
- **A file that does not parse** is never overwritten. A syntax error, a tab
  indent, a duplicate key or a YAML tag is reported at startup as `Settings:
  settings.yaml does not parse: …`, with the line and column of a syntax error
  and the path of the value otherwise (`options.Masks: !*.tmp is a YAML tag`).
  A value of the wrong type in an Options entry is reported the same way, and
  so is an empty value where a typed setting expects text
  (`command_line.prompt_format: invalid type: unit value, expected a string`).
  Save setup then stops with `Not saved: settings.yaml does not parse: …` and
  the exit auto-save writes nothing. A wrong value in a typed setting (`lsp`,
  `system`, …) makes FFM use the defaults for all typed settings and is only
  logged (with `FFM_LOG=1`); no save replaces it. Fix or remove the file, and
  the next save writes it again.
- **Migration from `settings.json`.** Older releases wrote `settings.json`, with
  the Options entries beside the typed keys. While there is no `settings.yaml`,
  FFM reads `settings.json` in its place with the entries moved under
  `options`, and the first save writes everything it held to `settings.yaml`.
  FFM never changes or removes `settings.json`; once `settings.yaml` exists, it
  is ignored. A save that cannot read `settings.json` names it:
  `Not saved: settings.json is not a regular file`.
- **Safety.** A save writes a temp file beside the settings and renames it over
  them, so a crash leaves the previous file whole; the file is readable by its
  owner only. A save holds a lock on `settings.yaml.lock` beside the file from
  reading it to the rename, so two FFMs saving at once keep each other's
  changes; while one saves, the other stops with `Not saved: another FFM is
  saving settings.yaml`. The empty lock file stays. FFM reads the file only
  when it is a regular file of at most 1 MiB.

## Verification

Workflow tests use temporary files and directories. Menu and renderer tests
cover executable entries, shortcuts, modal input, small terminals, view modes,
and mouse selection across columns. Host task/device queries are read-only;
tests do not resize the user's terminal or mutate host installations.
