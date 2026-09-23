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

Left and Right configure their respective panel. Brief, Medium, Full and Wide
are one table layout (`src/ui/panel.rs`): Brief and Medium flow the names down
three and two columns, Full shows name/size/date/time and Wide name/size. Each
column is titled, ruled and closed on the divider, and the info row adds the
current entry's size, date and time that the view leaves out. In Brief and
Medium `Left`/`Right` move the cursor a column and the mouse maps clicks through
the same geometry (`ui::stripe_at`). Detailed adds byte sizes and timestamps; Alternative full shows names
and readable sizes. Description modes read the configured description sidecar;
Long descriptions reserves more width for that text. Owners and Links show
cached filesystem ownership and link information, refreshed during navigation.

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
| Wipe | F8-style confirmation (Y/Enter wipes, N/Esc/F10 cancels); each file is overwritten with zeros, truncated, renamed to a temporary name and removed. Directories recurse; symlinks are unlinked, never followed. This does not guarantee erasure from SSDs, snapshots, or backups. |
| Add to archive | Far's dialog: name, format (tar.gz / tar / zip where `zip` is installed), "Delete files after archiving". Runs the host `tar` or `zip`; an existing archive is never overwritten. |
| Extract | Far's dialog for every marked archive, into the other panel by default (created if missing). Traversal paths, links and special archive members are refused. |
| Archive commands | Test archive, List contents, Extract here, Extract to other panel, Add selected files to this archive. |
| Attributes | Changes read-only/writable status; Unix also accepts octal permissions. |
| Apply command | Runs a shell template once per selected entry in panel order with [Far substitutions](apply-command.md): filenames and native short names, paths, panel prefixes and temporary selection lists. Every run is echoed into the Ctrl+O console; processed entries are unmarked and Ctrl+M restores the selection. |
| Describe | Updates the configured description sidecar while retaining other entries. |
| Find file/folder | Recursive name-mask search with configurable entry and depth limits. |
| Command history | Copies the selected command into the command line for review and execution. |
| File/folder history | Reopens a viewed file or previously visited directory. |
| Video mode | Requests 25 or 50 terminal rows, retaining width; host-terminal support is required. |
| Compare folders | Marks missing or different-sized files in both panels; this is not a recursive content comparison. |
| Edit user menu / associations | Opens configuration text in the built-in editor. |
| Folder shortcuts | Opens a saved bookmark; `:bookmark set N` creates one. |
| Panel filter | Filters files by mask; directories remain available for navigation. |
| Screens | Chooses file panels or console transcript. F9 from the console returns to panels and opens the menu. |
| Tasks / hotplug devices | Shows host process information or currently mounted drives. |
| Plugin commands | Opens the existing MCP status interface. |

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
| Languages | English or a JSON dictionary mapping menu row labels, including `&` mnemonic markers, to translated text |
| Plugins configuration | Additional command offered from the user menu |
| Plugins manager | Shell/plugin command timeout |
| Dialog / Menu | Escape dismissal / wrapping row navigation |
| Command line / AutoComplete | History length / completion and suggestions |
| InfoPanel | Folder descriptions in quick view |
| Groups of masks | Default semicolon-separated selection patterns |
| Confirmations | Delete confirmation |
| File panel modes | Default panel view |
| File / Folder descriptions | Sidecar filename / candidate folder-description filenames |
| Viewer / Editor | Tab display width / inserted indentation width |
| Code pages | Encoding for new files |
| Colors / Highlighting | Panel background / file-type colors |
| Save setup | Atomically persists settings for future sessions |

Settings are stored at `ffm/settings.json` beneath `APPDATA`, `XDG_CONFIG_HOME`,
or `HOME/.config`, in that order. User-menu and association files live beside
it. Sessions constructed without a configuration location use `.ffm` under the
active directory for command files. Settings are only persisted by Save setup.
Invalid settings loads preserve the existing settings and both panels.

## Verification

Workflow tests use temporary files and directories. Menu and renderer tests
cover executable entries, shortcuts, modal input, small terminals, view modes,
and mouse selection across columns. Host task/device queries are read-only;
tests do not resize the user's terminal or mutate host installations.
