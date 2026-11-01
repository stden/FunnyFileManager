<!-- Purpose: Captures the bash-grade command-line design spec used for implementation work. -->

# 🏛️ Bash-Grade `:` Command Line — Design

> Status: implemented. This file is the original design record. Current command-line behavior is in `docs/knowledge-base/07-core-features/command-line.md`.

**Date:** 2026-08-10
**Supersedes nothing;** extends `2026-07-25-smart-autocomplete-design.md`, which
shipped fuzzy matching, the dropdown, ghost text and bookmark sources.

## 🧐 Problem

The command line is a one-way tape. Concretely:

1. **The buffer is append-only.** `popup.rs` edits it with `push`/`pop` only.
   There is no cursor: a typo in the middle of `cd /very/long/path` costs the
   whole tail. `Home`/`End`/`Left`/`Right`/`Delete`/`Ctrl+W`/`Ctrl+U`/`Ctrl+K`
   do nothing a shell user expects.
2. **History is write-only.** `History::suggest` powers ghost text, but nothing
   walks the ring: `Up` does not recall the previous command and there is no
   `Ctrl+R` reverse search.
3. **Path completion is `cd`-only.** `complete_with` matches `Some(&"cd")` and
   returns `None` for every other argument position, so `cat src/ma<Tab>` and
   `ls /usr/l<Tab>` complete nothing.
4. **Executables never complete.** Typing `car<Tab>` offers verbs only; `cargo`
   is not a candidate because `$PATH` is never read.
5. **Completed paths are not shell-safe.** Outside `cd`, a candidate containing
   a space is spliced raw into a line handed to `sh -c`, where it becomes two
   arguments.
6. **Reaching the command line needs `:` first.** Bare letters resolve to
   `Action::QuickSearchClear` — they are swallowed. Far Manager and every shell
   start a command the moment you type.
7. **Command output is truncated to one line.** `dispatch.rs` keeps the last
   stdout line and throws the rest away; `ls -la` is unreadable.

## ⚖️ Decision

Grow the command line into a readline-grade editor, in three shippable phases,
without changing what a command *means* — `Command::parse`, the durability gate
in `history.rs`, and the shell timeout in `ops/shell.rs` are untouched.

### ⚖️ Approaches considered

1. **Extract a pure `LineEditor`, extend `complete`, keep the overlay model
   (chosen).** Cursor state lives in one dependency-free module that is
   exhaustively unit-testable, exactly as `complete/matcher.rs` already is. The
   overlay stays modal, so `dispatch_key`'s "overlay consumes every key" rule
   and all existing modal tests survive.
2. **Bring in `rustyline` / `reedline`.** Rejected: both own the terminal and
   the event loop. FFM's loop is a ratatui draw plus an *injected* event reader
   that every event-loop test depends on; handing input to a foreign readline
   would delete that seam. It also drops ~40 crates in for editing logic that
   is ~200 lines.
3. **Make the command line non-modal and always live (true Far Manager).**
   Rejected for now: panel keys and command-line keys would have to be
   disambiguated on every keystroke, which reworks `action.rs`, `lib.rs`
   dispatch and every modal test. `Action::CommandLineChar` (below) buys the
   same "just start typing" feel at a fraction of the blast radius.

## 🧱 Components

### 🪶 `src/lineedit.rs` — pure cursor-aware buffer (new)

```rust
pub struct LineEditor { text: String, cursor: usize, kill: String }
```

Fields are private because the type carries an invariant: `cursor` is a byte
index that is always a `char` boundary in `text` and never exceeds `text.len()`.
A public field would let any caller park the cursor mid-UTF-8 and panic the next
`insert`. Accessors: `text()`, `cursor()`, `is_empty()`.

Operations, all `O(len)` at worst and all pure:

| Method                                           | Meaning                                                |
| ------------------------------------------------ | ------------------------------------------------------ |
| `insert_char`, `insert_str`                      | splice at cursor, advance                              |
| `backspace`, `delete_forward`                    | remove one char before / after cursor                  |
| `left`, `right`, `home`, `end`                   | cursor moves, saturating                               |
| `word_left`, `word_right`                        | to the nearest word boundary                           |
| `kill_word_left`, `kill_to_start`, `kill_to_end` | cut into `kill`                                        |
| `yank`                                           | insert `kill` at cursor                                |
| `set_text`                                       | replace the whole line, cursor to end (history recall) |
| `clear`                                          | empty the line                                         |

Word boundary rule, matching bash's `\w` behaviour: a *word* is a maximal run of
non-whitespace. `word_left` skips whitespace backwards, then the run.

**No kill ring stack** — one slot. Bash's ring with `Alt+Y` rotation is a
feature nobody uses in a file manager's command line (KISS).

### 🏷️ `src/complete/shellwords.rs` — quoting-aware tokenizing (new)

Three pure functions:

- `last_word(input) -> (head, word)` — split at the start of the word under
  the end of `input`, respecting `'…'`, `"…"` and backslash escapes. Unlike
  `split_last_token`, `cd My\ Doc` yields the word `My\ Doc`, not `Doc`.
- `unescape(word) -> String` — the literal text a shell would see: strip one
  layer of quotes, resolve `\x` to `x`.
- `escape(text) -> String` — backslash-escape whitespace and the shell
  metacharacters `` " ' \ $ ` & | ; < > ( ) * ? [ ] { } # ~ ! `` when they would
  otherwise be reinterpreted. Round-trip property: `unescape(escape(s)) == s`
  for every `s`, asserted as a test.

Completion matches against `unescape(word)` and splices back `escape(candidate)`,
so a directory named `My Documents` completes into a line `sh -c` parses as one
argument.

`cd` keeps its existing whole-rest-of-line token rule and its quote stripping in
`command.rs` — that behaviour is specified, tested, and users have it in their
fingers. `cd` therefore does *not* escape; every other position does.

### 💡 `src/complete/execs.rs` — `$PATH` executables (new)

`executables() -> &'static [String]`: split `$PATH`, read each directory, keep
entries that are executable (`mode & 0o111` on Unix; any file on Windows),
deduplicate, sort. Memoized in a `OnceLock` — `$PATH` does not change inside a
process, and rescanning ~40 directories per keystroke would be felt.

Unreadable `$PATH` directories are skipped, never fatal — the same rule the rest
of completion already follows.

### ✏️ `src/complete.rs` — position rules (changed)

`complete_with` gains a real position model. Today it matches the first word
against a fixed list and gives up. The new rule set, in order:

| Position                                            | Candidates                                        |
| --------------------------------------------------- | ------------------------------------------------- |
| Word 0, token contains `/` or starts with `.` / `~` | paths (a command given by path)                   |
| Word 0, otherwise                                   | `VERBS` ∪ `$PATH` executables, verbs ranked first |
| Word ≥1 of `cd`                                     | paths (unchanged, unescaped, whole-rest token)    |
| Word 1 of `sort` / `bookmark` / `mark`              | sub-command words (unchanged)                     |
| Word 2 of `bookmark`                                | slot digits (unchanged)                           |
| Anything else                                       | **paths, shell-escaped**                          |

The last row is the change that makes `cat src/ma<Tab>` and `ls /usr/l<Tab>`
work. The path source itself (`path_complete`, fish-or-builtin) is reused as is.

**Resolution (phase 2):** a prefix matching exactly **one** built-in verb is
accepted outright and `$PATH` is not consulted at all. Without that rule `pw`
stops completing to `pwd ` the moment the host also has `pwdx`, turning an
accepted keystroke into a menu; the verbs are this command line's own closed
vocabulary, so a unique hit in them is the answer. When the prefix is ambiguous
among verbs — or matches none — executables join the list, always after the
verbs.

Tab completes **at the cursor**, not at the end of the line: the token under the
cursor is replaced and the tail after it is preserved. This is the one place the
cursor leaks out of `lineedit` into `complete`, and it is why `complete_with`
takes the text up to the cursor rather than the whole buffer.

### ✏️ `src/history.rs` — recall and search (changed)

Two additions; the `DurableLine` gate and the file format are untouched.

- `recall(index) -> Option<&str>` — the `index`-th entry counting back from the
  newest across session-then-durable, the same order `suggest` already walks.
- `search_back(query, from) -> Option<usize>` — the index of the first entry at
  or after `from` whose text *contains* `query`, for `Ctrl+R`. Substring, not
  fuzzy: `Ctrl+R` is bash's, and its result is shown before it is accepted.

### 🧩 `src/models.rs` — command-line state (changed)

```rust
pub struct CommandLine {
    pub line: LineEditor,          // was `buffer: String`
    pub menu: Option<Menu>,
    pub ghost: Option<String>,
    pub recall: Recall,
}

pub enum Recall {
    Idle,
    Browsing { index: usize, stash: String },
    Searching { query: String, hit: Option<usize> },
}
```

`Recall` is an enum, not a bag of `Option` fields, for the reason `Overlay` is:
browsing history and reverse-searching it are mutually exclusive, and the
`stash` — the half-typed line the recall displaced, restored on walking back
past the newest entry or on abandoning a search — is meaningless in `Idle`.

**Resolution (phase 1):** `Searching` carries its own `stash` rather than
borrowing `Browsing`'s. The design listed it only on `Browsing`, but `Esc`
during a search is specified to restore the pre-search line, which needs
somewhere to keep it; a single field on `CommandLine` would have been
meaningless in `Idle`, which is the thing the enum exists to prevent.

`From<&str> for CommandLine` and `From<String>` keep working, so the ~13 test
sites that build a command line from a literal do not move.

`get_command_line()` keeps returning `Option<&str>`; `get_command_line_mut()`
returns `Option<&mut LineEditor>` — a `&mut String` cannot be handed out any
more without letting a caller invalidate the cursor.

### ✏️ `src/popup.rs` — the key matrix (changed)

| Key                                           | Menu open           | Searching                | Otherwise                                                   |
| --------------------------------------------- | ------------------- | ------------------------ | ----------------------------------------------------------- |
| `Tab`                                         | next candidate      | —                        | complete at cursor                                          |
| `Up` / `Down`                                 | move selection      | —                        | history back / forward                                      |
| `Ctrl+R`                                      | —                   | previous match           | start reverse search                                        |
| `Enter`                                       | accept candidate    | accept match into buffer | execute                                                     |
| `Esc`                                         | close menu          | cancel, restore buffer   | close overlay                                               |
| `Left` / `Right`                              | —                   | —                        | move cursor; `Right`/`End` **at end of line** accepts ghost |
| `Home` / `Ctrl+A`, `End` / `Ctrl+E`           | —                   | —                        | line start / end                                            |
| `Alt+B` / `Ctrl+Left`, `Alt+F` / `Ctrl+Right` | —                   | —                        | word left / right                                           |
| `Backspace`, `Delete`                         | edit + refresh menu | edit query               | edit buffer                                                 |
| `Ctrl+W`, `Ctrl+U`, `Ctrl+K`                  | —                   | —                        | kill word back / to start / to end                          |
| `Ctrl+Y`                                      | —                   | —                        | yank                                                        |
| `Ctrl+C`                                      | —                   | cancel                   | clear line                                                  |
| `Ctrl+D`                                      | —                   | —                        | close overlay when the line is empty, else forward delete   |
| printable                                     | edit + refresh menu | extend query             | insert at cursor                                            |

`Right`/`End` are overloaded deliberately. They accept the ghost only when the
cursor already sits at the end of the line — which is exactly the state the
current implementation is always in, so the existing gesture is preserved while
mid-line cursor movement becomes possible.

### ✏️ `src/action.rs` — typing opens the line (changed)

New `Action::CommandLineChar(char)`: a printable character with neither Ctrl nor
Alt opens `Overlay::CommandLine` seeded with that character. `:` still opens an
empty one.

Two bindings must yield:

- **`q` / `Q` stop quitting.** They are the first letters of `quit`, `qemu`,
  `queue`; a command line you cannot type `q` into is not a command line. Exit
  remains `F10`, `Ctrl+C`, and the `quit` / `exit` commands. This is a
  deliberate, user-approved break with the current keymap.
- **`Space` keeps `ToggleMark`.** A command never usefully starts with a space,
  and marking with `Space` is load-bearing Far Manager muscle memory. Once the
  command line is open, `Space` types normally like any other character.

### ✏️ `src/ops/dispatch.rs` — command output (changed)

A shell command whose combined output is **more than one non-empty line** opens
`Overlay::Viewer` titled with the command, holding stdout then stderr, scrollable
with the keys `F3` already uses. One line or none keeps today's status-bar-only
behaviour — popping a full-screen viewer for `pwd` would be worse than the bug.

Status text is unchanged in all cases, so a user who dismisses the viewer still
reads the summary.

### ✏️ `src/ui.rs` — rendering (changed)

- The cursor is drawn **where it is**: the visible text splits at
  `cursor()` and the character under the cursor renders reversed (or a reversed
  space at end of line), replacing today's trailing `"_"`. The existing
  cell-budget windowing is kept but re-anchored so the cursor is always inside
  the visible window.
- While `Recall::Searching`, the prompt is replaced by
  `(reverse-i-search)`query': ` and the matched line renders as the buffer,
  matching bash.
- Ghost text is unchanged, and is suppressed whenever the menu is open or a
  search is active.

## 📌 Phasing

1. **Line editing.** `lineedit.rs`, the `CommandLine`/`Recall` state change,
   the readline key matrix, cursor rendering, history `Up`/`Down` and `Ctrl+R`.
   No completion changes. Independently shippable and reviewable.
2. **Completion.** `shellwords.rs`, `execs.rs`, the position rules, completing
   at the cursor, shell-escaped insertion.
3. **Entry and output.** `Action::CommandLineChar` (with the `q` and `Space`
   resolutions), and the viewer for multi-line command output.

Phase 1 carries the widest ripple (every site touching `state.buffer`), so it
gets its own review rather than being buried under the completion diff.

## 🔑 Secrets

Unchanged and load-bearing: `DurableLine::classify` still gates what reaches
disk, and `Command::Shell` / `Command::Invalid` are still session-only. The two
new history entry points read the existing rings and add no writer, so no new
path can persist a line the gate rejects. `Ctrl+R` can surface a session-only
shell line within a run — the same exposure ghost text already has, and it dies
with the process.

## 🪶 Known simplifications (KISS)

- **No interactive TTY handover.** `vim`, `htop` and `less` still run with piped
  stdio under the 10-second cap. Giving a child the real terminal means
  suspending the TUI, and `TerminalSuspension` already exists for `Alt+F4` — but
  wiring it to arbitrary commands needs its own cancellation and resize story.
- No kill-ring rotation (`Alt+Y`), no `Ctrl+X` prefix commands, no vi mode.
- No history expansion (`!!`, `!$`, `^old^new`).
- No brace/variable/glob expansion in completion — `sh -c` still does all
  expansion at execution time, as today.
- No multi-line commands, no line continuation.
- No completion inside the F7 mkdir / Shift+F4 new-file dialogs (unchanged).
- No S3 or remote-path completion (unchanged).

## 🚨 Error handling

Unchanged in spirit: completion and history must never break the file manager.

- An unreadable `$PATH` directory contributes nothing; the scan continues.
- `escape`/`unescape` are total — no input is rejected, so a pathological token
  degrades to a literal, never to an error.
- Cursor operations saturate rather than panic; the char-boundary invariant is
  asserted with `debug_assert` at every mutation.
- A history recall past either end of the ring is a no-op that leaves the buffer
  alone.

## 🧪 Testing

- **`lineedit.rs`** — pure unit tests: insert/delete at every position, word
  moves across runs of whitespace, kill/yank round-trip, and a **UTF-8 test
  moving the cursor through CJK and combining characters** so a byte-index bug
  cannot survive.
- **`shellwords.rs`** — `unescape(escape(s)) == s` over a fixed adversarial
  corpus (spaces, quotes, backslashes, `$`, `*`); `last_word` on quoted and
  escaped input.
- **`execs.rs`** — `TempDir` `$PATH` with one executable and one non-executable
  file, asserting only the executable is offered.
- **`complete.rs`** — `TempDir` panel: completing an argument of an arbitrary
  command; completing at a cursor in the middle of a line preserves the tail; a
  candidate containing a space comes back escaped.
- **`history.rs`** — `recall` walks session before durable and stops at the end;
  `search_back` finds a substring and skips past `from`.
- **`popup.rs`** — the key matrix above in all three column states, including
  `Esc` restoring the pre-search buffer and `Right` at end-of-line still
  accepting the ghost.
- **`action.rs`** — a bare letter resolves to `CommandLineChar`; `Space` still
  resolves to `ToggleMark`; `F10` still quits and `q` no longer does.
- **`ui.rs`** — `TestBackend` assertions for the cursor rendered mid-line and
  for the reverse-search prompt.
- **`dispatch.rs`** — a two-line command opens the viewer; a one-line command
  does not.
