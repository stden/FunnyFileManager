# Session tools

Open **F9 → Commands** for action statistics, package updates, and separate PATH
editors for the current user and all users.

## Panel directories

FFM automatically restores the last directories shown in its left and right
panels. Navigation and panel swaps save the pair before the next screen is
displayed, so closing the terminal window also keeps the last displayed
locations. Unchanged panels do not trigger another write. With several FFM
windows, the most recently saved navigation wins.

The state is written atomically to `%APPDATA%/ffm/panel-session.json` on Windows
or `$XDG_CONFIG_HOME/ffm/panel-session.json` (`~/.config/ffm/panel-session.json`
by default) on Unix. This is independent of **Auto save setup**. If a saved
directory is missing or unreadable, only that panel falls back to the launch
directory. Missing or invalid session files start both panels there. Delete
the session file while FFM is closed to reset the remembered directories.

## Action statistics

The `stats` command opens the same report as **Action statistics**. It ranks
actions by total waiting time and shows count, average, maximum and rendering
time in milliseconds. Timing starts after input arrives and finishes after the
resulting frame is drawn; time spent deciding what to type is excluded. WSL
background requests have separate `Background/WSL/...` totals, including queue
time. Those totals may overlap foreground actions and should not be added to
them as a wall-clock session duration. Package updates include interactive
manager prompts in their elapsed operation time.

At exit, including an event-loop error, aggregates are saved atomically in
`%APPDATA%/ffm/performance/` on Windows or `$XDG_CONFIG_HOME/ffm/performance/`
(`~/.config/ffm/performance/` by default) on Unix. Each session has a separate
JSON file. These files contain action categories and timings, not typed text,
file paths, command arguments or passwords. The on-screen report covers the
current session; JSON files can be compared across runs. This measures user
waiting, not CPU time, and is separate from opt-in raw event recordings.

## Package updates

**Update packages** detects Chocolatey on Windows, Homebrew on macOS, and
apt-get (then Homebrew) on Linux. It shows the commands and defaults to Cancel:

| Manager | Commands |
|---|---|
| Chocolatey | `choco upgrade all -y` |
| apt-get | `apt-get update`, then `apt-get upgrade -y` |
| Homebrew | `brew update`, then `brew upgrade` |

Select Run to update all installed packages managed by that tool. The terminal
is handed to the package manager for progress and prompts, then restored.
Chocolatey requests Windows elevation. apt-get run by a user other than root
goes through sudo, and the Run row says so: `Run: sudo apt-get update && sudo
apt-get upgrade -y`. Each step is echoed into the Ctrl+O transcript with what it
wrote to stderr, which also still appears on the terminal as it runs.

A failed step stops the sequence. The status names the program that failed,
its exit code and the last line it wrote to stderr, for example
`apt-get update exited with code 100: E: Could not get lock ...`. When sudo
itself refuses (wrong or missing password, not in sudoers, no terminal) it
exits 1, which apt-get never uses, and the status says so:
`sudo did not run apt-get update (exit code 1): sudo: a password is required`.
Ctrl+C at sudo's password prompt, or during the update, stops the package
manager and not FFM: the panels come back with "Package update cancelled". This does not install a
missing package manager or assume that FunnyFileManager itself was installed
through one.

## PATH editor

Use **Ins / F7** to add, **Enter / F4** to edit, **Delete / F8** to remove, and
**Ctrl+Up / Ctrl+Down** to reorder directories. Paths are checked when opened,
accepted, and saved; **F5** checks again. Missing directories, files, relative
paths, empty components and unknown environment variables are marked. Fix or
remove invalid rows before saving. **F2** asks to save the selected scope;
**Esc** asks to discard a changed draft.

Windows reads the separate user and machine registry values without expanding
away references such as `%SystemRoot%`. Saving keeps an expandable registry
type where needed and broadcasts an environment-change notification. System
PATH requires running FFM with administrator rights.

On Unix, user PATH is saved in an FFM block in the login profile (`.zprofile`
for zsh, otherwise the bash login profile or `.profile`). System PATH uses
`/etc/environment` on Linux and `/etc/paths` on macOS. Existing unrelated
settings are retained and the previous file is backed up as `*.ffm-path.bak`.
System writes require root. Shell configuration loaded later can override
these values. The editor refuses an external change made since it opened.
Open a new terminal/login session to pick up saved PATH; an already-running
process keeps its inherited environment.

## MCP server setup

**F9 → Commands → Install MCP server** (hotkey **e**) registers `ffm-mcp`,
FFM's MCP server, with Claude Code, Codex, Junie and Kiro. The dialog shows
where the server was found, a scope switch, one checkbox per client (whether
the client was detected, and what its config says now), and notes for the
chosen scope.

Keys: **Tab / Down** and **Shift+Tab / Up** move between the scope switch,
the client rows and the buttons; **Left / Right** on the scope switch change
scope; **Space** ticks or clears a client; **Enter** registers the ticked
clients (on **Cancel** it cancels); **Esc / F10** close without changes. A
click on Register or Cancel does the same as Enter on it.

The server is looked for next to the running `ffm` first — where
`cargo install --path mcp` puts it — then on PATH. When it is not found, the
dialog says so, every row is dimmed, and nothing can be registered; FFM never
builds it.

Two scopes:

- **All projects** (the default) writes each client's user-level config. The
  entry carries no `FFM_MCP_ROOT`, so the server confines itself to whichever
  folder the client starts it in.
- **Current folder** writes project-level configs inside the active panel's
  folder, with `FFM_MCP_ROOT` set to that folder. It is not available when the
  active panel is not a local folder (a NetBox panel, an S3 listing, a folder
  that has gone).

| Client | All projects | Current folder |
|---|---|---|
| Claude Code | `claude mcp add --scope user --transport stdio ffm-mcp -- <server>`, run in Claude's home folder. An entry that differs is replaced with `claude mcp remove --scope user ffm-mcp`, then `claude mcp add-json --scope user ffm-mcp '<entry>'` carrying the old entry's `env` and other keys. The user config is only read, found where the CLI finds it: a legacy `.config.json` in `$CLAUDE_CONFIG_DIR` (else `~/.claude`) when there is one, else `.claude.json` in `$CLAUDE_CONFIG_DIR` (else the home) | `<folder>/.mcp.json`, edited by FFM |
| Codex | `$CODEX_HOME/config.toml`, else `~/.codex/config.toml` | `<folder>/.codex/config.toml` |
| Junie | `~/.junie/mcp/mcp.json` | `<folder>/.junie/mcp/mcp.json` |
| Kiro | `$KIRO_HOME/settings/mcp.json`, else `~/.kiro/settings/mcp.json` | `<folder>/.kiro/settings/mcp.json` |

JSON configs get `{"mcpServers": {"ffm-mcp": {"command": "<server>", "args": [],
"env": {}}}}`; Codex gets `[mcp_servers.ffm-mcp]` with `command` and
`args = []`, plus `[mcp_servers.ffm-mcp.env]` in a folder. An entry counts as
registered when its command is this server, it passes no arguments, nothing
starts it another way (no `url`, no `cwd`, no `type` but `stdio`), and its
`FFM_MCP_ROOT` matches the scope; its other keys are its owner's. Registering
over a different entry removes `url`, `cwd` and a non-stdio `type`, and keeps
the rest. An entry switched off (`"disabled": true`, `enabled = false`) shows
as "registered (disabled)" and stays off. When the server is not found, an
entry shows only as present — or as running a missing program, when its
command is an absolute path to nothing.

Current folder is refused for a client whose project config there is its
user-level config: with the panel on the home folder, `~/.codex/config.toml`,
`~/.junie/mcp/mcp.json` and `~/.kiro/settings/mcp.json` are both at once, and
an entry confined to the home would reach every project. Those rows read
"same file as All projects".

Configs are never clobbered:

- A config that does not parse, or has an unexpected shape, is dimmed in the
  dialog, left untouched, and named in the report with the parser's line and
  column. Fix or remove it, then register again.
- Every other server, key, comment and (in JSON) key order is kept. In JSON
  every value but the `ffm-mcp` entry keeps its exact text — `1e3` stays
  `1e3` — the file's indentation and CRLF line ends are kept too. Codex's
  file is edited with `toml_edit`, so inline tables and dotted keys keep their
  form.
- Only a regular file of at most 64 MiB is read: a FIFO or a device where a
  config should be is reported, not opened.
- The new text is complete before anything is replaced, and the file is
  replaced in one atomic rename that keeps its permissions; a symlinked
  dotfile keeps its link. A new file is created owner-only. No `.bak` is left
  (beside `.mcp.json` it would land in the repository). If the client saves
  the file between FFM's last read and the rename, the rename is dropped and
  the report asks to register again.
- Registering again when nothing changed writes nothing.
- The `claude` CLI runs without a terminal, bounded to 30 seconds a call,
  output captured, and never against a `.claude.json` that does not parse.
  While it runs, the dialog says so in place of its notes. At the time limit
  the CLI is stopped with everything it started (its process group on Unix,
  a job on Windows), so a shim cannot leave the real CLI running. Its
  success is checked by reading the user config again. Without `claude` on
  PATH, Claude Code's All projects row cannot be changed. When a step fails,
  the report gives the command to finish by hand.
- One client failing does not stop the others. The "MCP server registration"
  report lists each client's outcome (added, updated, unchanged, failed) with
  the file or the command, and the status line sums it up.
- A registered row is shown ticked and dimmed: this dialog never removes a
  registration.

Client notes:

- **Junie and Kiro**, All projects: they may start the server outside the
  project, where it confines itself to the wrong folder; prefer Current
  folder.
- **Claude Code**, Current folder: it asks once before it starts a server
  from `.mcp.json`.
- **Codex**, Current folder: it reads `.codex/config.toml` only in folders it
  trusts.
- Claude Code's local scope (`projects[<folder>]` in `~/.claude.json`, what
  `claude mcp add` without `--scope` writes) overrides the user entry in that
  folder; FFM neither reads nor writes it.

Restart running clients, or reload their MCP servers, to pick up a change.
**F11 → MCP status** (or `:mcp`) shows the same states for both scopes.

## KeePass

KeePass is enabled in default builds. Both `.kdbx` and `.kbdx` are accepted,
case-insensitively. **F3 / Enter** opens a password prompt in view mode; **F4**
opens edit mode. Password input is masked and incorrect input is cleared.

After unlocking, the full group tree (including empty groups) is the default.
Type to filter by substring across title, username, URL, or group path.
**Enter** copies the selected password, **Ctrl+U** copies its username,
**Ctrl+R** opens a password dialog, and **F6** switches between tree and list.

In edit mode, **F4** opens the selected entry. **Tab / Shift+Tab** selects title,
username, URL, notes or password; typing and Backspace edit that field. The
password is visible and **Ctrl+C** copies it. **Ctrl+Z** undoes the latest field
change. **F2** saves; **Esc** asks whether to save a changed draft. Entry identity is kept,
so editing one of two entries with the same title affects the selected entry.
The storage adapter keeps an encrypted `.bak` and replaces the database
atomically. **F8** confirms and deletes an entry; **Ctrl+Z** in the browser
restores the latest saved edit or deletion from that backup. The upstream writer supports KDBX4 and does not preserve every
extension field; unsupported writes report an error and retain the draft.

**Esc** from the entry list remembers the key only in this FFM process.
Reopening rereads the database using that key. Five minutes of vault inactivity
forgets it; **Ctrl+L** immediately locks and forgets all cached keys. Keys are
never saved to settings or statistics. Copied secrets have a 30-second clipboard
clear timer. Creating or deleting groups is outside this editor's scope.

## TUI BDD

`cargo test --features test-helpers --test tui_bdd` runs Gherkin scenarios from
`tests/tui_features/` through the production event loop, key dispatch and
ratatui renderer. Tests inspect rendered cells and isolated fixture files;
the clipboard is an in-memory adapter. No test updates host packages or saves
host PATH. The standard Rust test harness wraps Cucumber so nextest and coverage
run it too.

GitHub Actions runs the suite on Ubuntu (inside the coverage job) and on
Windows (inside the full Windows suite) for pushes to main/master and pull
requests. Frame transcripts are uploaded as `tui-screens-Linux` and
`tui-screens-Windows` artifacts, including failed scenarios. Locally they are
in `target/tui-bdd/` or the directory set by `FFM_TUI_ARTIFACT_DIR`.
