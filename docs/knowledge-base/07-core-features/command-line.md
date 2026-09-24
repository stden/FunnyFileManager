<!-- Purpose: Describes the typed command line, modal input, and shell fallback. -->

# 💻 Command Line

Pressing `:` in the main view opens an empty
`Overlay::CommandLine(CommandLine)`; **any other printable key opens it already
holding that character** (`Action::CommandLineChar`), the way Far Manager and
every shell start a command. `Space` still marks and `Alt+letter` is still quick
search. Bare `q` no longer quits — see
`docs/knowledge-base/06-ui-and-interaction/keyboard-handling.md`.

`CommandLine` owns the editable line, the completion-menu state, the ghost
suggestion and the history-recall mode. The command line is strictly modal:
while it is open, every key is consumed before `Action::resolve`. Unsupported
keys leave its state unchanged.

Responsibilities are split as follows:

- `src/command.rs` — `Command` domain enum and pure `Command::parse`
- `src/lineedit.rs` — pure `LineEditor`: text, caret, and one kill slot
- `src/popup/command_line.rs` — the readline keymap: editing, completion-menu
  navigation, history recall, reverse search, close, and submit behavior
- `src/complete.rs` — fuzzy verb and path candidates, position rules
- `src/complete/shellwords.rs` — quote-aware word splitting and escaping
- `src/complete/execs.rs` — `$PATH` executable names
- `src/history.rs` — persistence, ghost lookup, recall, reverse search
- `src/ops/dispatch.rs` — execution of parsed commands
- `src/ops/shell.rs` — platform shell fallback

## 🪟 Modal Keys

The line is edited with a caret, not appended to. Three key columns apply
depending on whether the completion dropdown is open and whether a `Ctrl+R`
reverse search is running.

| Key | Dropdown open | Searching | Otherwise |
|---|---|---|---|
| `Tab` | next candidate | — | complete the token at the caret |
| `Up` / `Down` | move selection | — | history back / forward |
| `Ctrl+R` | — | previous match | start reverse search |
| `Enter` | accept candidate | accept match into the line | execute |
| `Esc` | close dropdown | cancel, restore the typed line | close the overlay |
| `Left` / `Right` | — | — | move caret; `Right`/`End` **at end of line** accept the ghost |
| `Home` / `Ctrl+A`, `End` / `Ctrl+E` | — | — | line start / end |
| `Alt+B` / `Ctrl+Left`, `Alt+F` / `Ctrl+Right` | — | — | word left / right |
| `Backspace`, `Delete` | edit + refresh candidates | edit query | edit the line |
| `Ctrl+W`, `Ctrl+U`, `Ctrl+K` | — | — | kill word back / to start / to end |
| `Ctrl+Y` | — | — | yank the killed text |
| `Ctrl+C` | — | cancel | clear the line |
| `Ctrl+D` | — | — | close when empty, else forward delete |
| printable | edit + refresh candidates | extend query | insert at the caret |

Every other key is consumed without changing the line. It cannot trigger a panel
action such as F8, Ctrl+U, or F10.

`Up` stashes the half-typed line before the first recall and restores it on
walking forward past the newest entry, so history browsing is never destructive.
Multiple fuzzy matches remain in the dropdown instead of replacing application
status with an ambiguity report.

## 💬 Supported Built-in Commands

| Command | Description | Example |
|---|---|---|
| `cd <path>` | Change the active directory; relative, `~`, quoted, and space-containing paths are supported | `cd "My Documents"` |
| `pwd` | Put the active path in status | `pwd` |
| `sort name\|ext\|extension\|size` | Sort and reload the active panel | `sort size` |
| `bookmark set <0-9>` | Save the active path | `bookmark set 3` |
| `bookmark go <0-9>` | Open a saved path | `bookmark go 0` |
| `mark all` | Mark all visible entries except `..` | `mark all` |
| `unmark` | Clear active-panel marks | `unmark` |
| `help` | Open the help overlay | `help` |
| `mcp` | Open MCP status in the viewer overlay | `mcp` |
| `sessions` | Show NetBox's stored-sessions panel in the active panel | `sessions` |
| `sessions import netbox [path]` | Import a `.netbox` session export; the path is the rest of the line (relative to a file panel, `~` expanded), else the `.netbox` file under a file panel's cursor. Counts go to the status bar, each skipped entry and its reason to the console transcript | `sessions import netbox ~/demo.netbox` |
| `winmsg` / aliases | Open Windows system-message diagnostics | `winmsg` |
| `winperf` / `wincheck` | Open Windows performance diagnostics | `winperf` |
| `wsl` | Open the WSL manager on Windows | `wsl` |
| `quit` / `exit` | Set the application quit flag | `quit` |

Bare `sort` means sort by name. Rejected recognized forms—such as bare `cd`, an
unknown sort mode, or an invalid bookmark slot—become `Command::Invalid` with
command-specific usage text; an unknown verb becomes `Command::Shell` with the
original input. Completion and parsing share the command vocabulary rather than
maintaining separate verb lists.

Only bare `wsl` selects the manager. Argument-bearing calls such as
`wsl --list --verbose` and explicit `wsl.exe` invocations retain their original
text as `Command::Shell`; the built-in does not intercept them.

## 💡 Completion Positions

`Tab` completes the token **under the caret**, preserving the rest of the line.
Which source applies depends on the position:

| Position | Candidates |
|---|---|
| Word 0 containing `/`, or starting with `.` / `~` | paths (a program named by location) |
| Word 0 otherwise | built-in verbs, plus `$PATH` executables by prefix |
| Argument of `cd` | paths, unescaped (see below) |
| Argument 1 of `sort` / `bookmark` / `mark` | that verb's sub-commands |
| Argument 2 of `bookmark` | slot digits, populated slots hinted with their path |
| Built-in verb in a position it does not model | nothing |
| Any argument of anything else | paths, shell-escaped |

A prefix matching exactly one built-in verb is accepted outright and `$PATH` is
not consulted, so `pw` still completes to `pwd ` on a host that also has `pwdx`.
Otherwise the verbs are listed first and executables after — a built-in is never
outranked by a program that merely has a shorter name. `$PATH` executables match
by **prefix**, not fuzzily: a subsequence match on two characters returns most of
`$PATH`.

Candidates outside `cd` are backslash-escaped (`src/complete/shellwords.rs`), so
a directory called `My Documents` reaches `sh -c` as one argument. `cd` is the
exception in both directions: it takes the whole rest of the line as one
argument and `Command::parse` strips quotes rather than resolving escapes, so
its candidates stay literal.

## 💻 Shell Fallback

On Windows, `codex [arguments]` opens a separate interactive console in the
active panel directory. `codex.exe` and `codex.cmd` work the same way, including
case-insensitive spelling. The original argument text and quotes are passed to
`cmd /D /S /K`; for example, `codex -C "My Project" "Explain this code"`.
FFM remains usable immediately, and the normal shell timeout does not apply.
The new console has its own input and output and stays open after Codex exits,
so launch errors remain visible. Its launch is recorded in FFM's transcript.
On Unix, these commands retain normal shell execution.

Unknown commands run in the active panel directory:

- Unix: `sh -c <command>`
- Windows: `cmd /D /C <command>` with `CREATE_NO_WINDOW`

Execution is capped at ten seconds (F9 Options, Plugins manager settings) and
can be cancelled sooner with `Esc` or Ctrl+C. Either way the whole process
group is killed. The two endings are reported apart, in the status bar and the
transcript: `cancelled: <command>` for a cancel, which is not an error, and
`stopped '<command>': timed out after Ns` for the limit. Only a shell that
could not be started reads `cannot run '<command>': <reason>`; when that is
because the panel folder was deleted, the reason is `folder <path> no longer
exists` rather than the OS's "No such file or directory". Other keys read
while the command is running are discarded so they cannot replay against the
panels afterward.

After a process is launched, status uses the last stdout line that still shows
something once escape sequences are stripped, otherwise the first such stderr
line, otherwise `OK: ...` for success or the exit status for failure. CSI, OSC
and ECMA-35 sequences such as the `ESC ( B` charset switch from `tput sgr0`
and ncurses programs are removed from both the status and the transcript.

Everything the command wrote goes to the **console transcript** (`src/console.rs`)
— the layer drawn behind the panels, revealed with `Ctrl+O`, where the prompt
stays usable so the next command's output appears directly above it. See
`docs/knowledge-base/06-ui-and-interaction/rendering-and-layout.md`. No window
pops up over the panels: the status bar summary is enough for the common case,
and the full output is one keystroke away when it is not.

A panel refresh is attempted, and its error is currently ignored. Failure to
launch the shell command returns an error to popup handling.

## 🪟 Error and Overlay Behavior

The command-line overlay closes before dispatch. `popup` translates a returned
execution error into `Status::Error`; commands that open another overlay may
replace the just-closed command line with that overlay. An empty submitted
buffer simply closes without dispatch.

## 🔗 Related Code

- `src/command.rs`
- `src/lineedit.rs`
- `src/popup.rs`, `src/popup/command_line.rs`
- `src/complete.rs`, `src/complete/shellwords.rs`, `src/complete/execs.rs`
- `src/history.rs`
- `src/console.rs`
- `src/ops/dispatch.rs`
- `src/ops/shell.rs`
- `src/handlers.rs` (compatibility re-exports only)
- `docs/knowledge-base/03-rust-best-practices/type-system.md`

The `:` command line is the only bookmark interface; there is no bookmark
overlay and no F2 binding. `bookmark set` writes the slot at once to
`ffm/bookmarks` in the data directory, beside the histories — one
`<slot> <path>` line per saved slot, owner-only, re-read before each write so
two running instances keep each other's slots — and startup loads it. MCP and Windows diagnostic commands reuse
`Overlay::Viewer` for rich output; ordinary shell output does not — it goes to
the console transcript instead.
