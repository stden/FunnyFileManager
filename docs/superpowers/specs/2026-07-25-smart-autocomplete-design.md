<!-- Purpose: Captures the smart autocomplete design spec used for implementation work. -->

# Smart Autocomplete for the `:` Command Line — Design

**Date:** 2026-07-25
**Problem:** Tab completion on the `:` command line (`src/complete.rs`) is
prefix-only, offers no visible candidate list beyond a space-joined status-bar
string, and forgets everything between runs. Typing `Downloads` requires typing
`Dow`; a command used five minutes ago must be retyped in full.

## Decision

Grow completion into three capabilities, sharing one ranked candidate pipeline:

1. **Fuzzy matching + a dropdown menu.** `dwn` matches `Downloads/`. Candidates
   render in a navigable popup instead of a status-bar string.
2. **History-aware ghost text.** The most recent matching past command shows
   greyed ahead of the cursor while typing; `Right`/`End` accepts it.
3. **Context-aware candidates.** Completion draws on bookmark slots that
   actually hold a path, showing the path as a dropdown hint. (The originally
   planned *active panel entries* source was dropped in phase 3 — see the
   `sources.rs` resolution note below.)

The dropdown opens only on explicit `Tab`; ghost text is always on while typing.
This keeps existing muscle memory intact — `Tab` still completes, and a user who
ignores the new features sees only the greyed suggestion.

### Approaches considered

1. **Layered modules under `complete/`, delivered in three phases (chosen).**
   Roughly 550 lines of production code across four concerns with genuinely
   different reasons to change — one pure, one reading `App`, one touching disk.
   The ~500-line file guidance forces a split regardless.
2. Grow `complete.rs` in place. Rejected: merges four concerns in one file,
   exceeds the size guidance, and makes the pure matcher untestable without
   dragging in `App` and the filesystem.
3. Auto-opening menu on every keystroke (IDE style). Rejected: a directory scan
   per keystroke, and the popup covers the panels most of the time.

## Components

> **As shipped (phases 1–3):** the "layered modules under `complete/`" plan was
> simplified. Only the pure matcher was split into `src/complete/matcher.rs`;
> the candidate **sources** (verbs, sub-commands, `cd` paths, bookmark slots)
> live in `src/complete.rs` itself, and **history** became a top-level
> `src/history.rs` (not `complete/history.rs`) since `App` — not the completer —
> owns it. The bullets below describe each concern's logic; read the file names
> as logical groupings, not the literal tree.

- **`complete.rs` — orchestration and sources.** Public entry points:
  `complete(input, cwd)` (thin wrapper) and `complete_with(input, ctx)` where
  `ctx: CompletionContext { cwd, bookmarks }`. Splits the token, picks the
  source that applies at that position, ranks through `matcher`, and returns a
  `Completion { line, candidates: Vec<Candidate> }` — each `Candidate` carries
  its own `text`, `kind` (trailing-space rule) and optional `hint`.

- **`complete/matcher.rs` — pure ranking, no dependencies.**
  `score(query, candidate) -> Option<Score>`; `None` means no match.
  Subsequence matching with bonuses for consecutive runs, word boundaries
  (after `/`, `_`, `-`, `.`) and prefix position, minus a gap penalty.
  **Smartcase:** an all-lowercase query matches case-insensitively; any
  uppercase character makes the whole query case-sensitive.
  `Score` carries a **tier** so exact-prefix matches always outrank fuzzy ones —
  typing `cd` must never be outranked by a fuzzy hit on `bookmark`.
  Ordering is `(tier, score desc, length asc, lexicographic)`: total and
  deterministic, because a ranking that varies between runs cannot be tested.

- **`complete/sources.rs` — candidate providers.**
  Existing sources (verbs, sub-commands, `cd` paths via fish-or-builtin-scan),
  plus *bookmarks* for `bookmark go`, offering only slots that hold a path and
  carrying that path as the candidate's `hint`.
  **`VERBS` is deleted** and the verb list derived from one place shared with
  `Command::parse` (shipped in phase 1).

  **Resolution (Phase 3):** the originally-planned *active panel entries*
  source was **dropped**. `complete()` is always called with the active panel's
  directory as `cwd`, and `cd` completion already scans that directory — so
  panel entries would duplicate that scan (they are the same directory's
  contents), a KISS violation with no user-visible gain. `cd` is the only path
  argument in the command DSL, so panel entries had no other command position
  to serve. Phase 3 shipped the bookmark-slot source only, threaded through a
  `CompletionContext { cwd, bookmarks }` and `Vec<Candidate>` completion
  results so each candidate carries its own `hint`.

- **`complete/history.rs` — persistence and ghost lookup.**
  File: `$XDG_DATA_HOME/ffm/history`, defaulting to `~/.local/share/ffm/history`,
  created mode `0600` on Unix. Capped at 500 entries, deduplicated keep-newest,
  newest last. `suggest(prefix)` returns the most recent entry having `prefix`
  as a **literal** prefix (not fuzzy — ghost text must be predictable, since
  `Right` accepts it without confirmation).

- **`models.rs` — richer command-line state.**
  ```rust
  Overlay::CommandLine(CommandLine)   // was Overlay::CommandLine(String)

  pub struct CommandLine {
      buffer: String,
      menu: Option<Menu>,      // None = dropdown closed
      ghost: Option<String>,   // suggestion tail beyond `buffer`
  }
  pub struct Menu { candidates: Vec<Candidate>, selected: usize }
  pub struct Candidate { text: String, kind: Kind, hint: Option<String> }
  ```
  This is the widest ripple in the change: `get_command_line`,
  `get_command_line_mut`, `popup.rs`, `ui.rs` and the handler tests all touch
  the current `String` payload.

- **`popup.rs` — key handling in the command-line overlay.**

  | Key | Menu closed | Menu open |
  |---|---|---|
  | `Tab` | open menu, advance to longest common prefix | next candidate |
  | `Up` / `Down` | — | move selection |
  | `Enter` | execute command | accept candidate, close menu |
  | `Esc` | close overlay | close menu only |
  | `Right` / `End` | accept ghost text | — |
  | printable / `Backspace` | edit buffer, refresh ghost | edit buffer, refresh menu |

  `Esc` closing only the menu — not the whole overlay — means an accidental
  `Tab` never costs a typed command.

  **A single candidate is accepted directly and opens no menu**, matching
  today's behaviour; the menu appears only when `Tab` finds two or more.
  **Ghost text is suppressed while the menu is open**, so there is never a
  greyed suggestion competing with a highlighted candidate for the same
  `Enter`. `Kind` keeps its current `Word` / `Path` meaning (it decides the
  trailing space); anything a candidate wants to *display* goes in `hint`.

- **`ui.rs` — rendering.** Ghost text renders in `render_command_line` as a
  `DarkGray` span appended after the buffer. The menu renders anchored above the
  command line, at most 10 rows, scrolling to keep the selection visible, with
  `hint` right-aligned.

  **Resolution (Phase 2, Task 4):** this section originally said ghost text
  shows "greyed ahead of the cursor" — read as buffer, then cursor glyph,
  then ghost. The shipped span order is buffer, ghost, cursor glyph instead,
  because the `"_"` cursor is a fake caret (the command line has no mid-line
  editing) rather than a real terminal cursor overlay. Trailing it after the
  ghost previews the post-`Right`-accept state instead of splitting the
  suggestion; a cursor-in-the-middle rendering (`"cd_ /somewhere"`) reads as
  broken text, not a continuation of what was typed.

## Phasing

Each phase is independently shippable and reviewable.

1. **Fuzzy + dropdown.** `matcher.rs`, the `CommandLine` state change, menu
   render and keys. Existing sources keep working, now ranked and displayed.
2. **History + ghost text.** `history.rs`, redaction rule, ghost render and
   `Right`/`End` acceptance.
3. **Context sources.** Bookmark slots (with path hints); `VERBS` already
   shared from phase 1. Panel entries dropped — see the `sources.rs`
   resolution above.

Phase 2 is the one with an on-disk artifact and the secrets rule, so it gets its
own review rather than being buried in a larger diff.

## Secrets

`src/keepass.rs` sets the rule that secrets must never reach files on disk, and
anything typed at `:` that is not a known verb becomes `Command::Shell` — so a
token or password in a shell command is exactly the kind of line that must not
be persisted.

**Only lines whose `Command::parse` yields a known verb are written to disk.**
`Command::Shell` and `Command::Invalid` are held in a session-only in-memory
ring, so ghost text still works for them within a run but nothing survives exit.
There is no regex guessing at what "looks secret" — the parse result is the
gate, which is testable and has no false negatives.

## Known simplifications (KISS)

- No S3 completion. The S3 client is blocking, so completing bucket or prefix
  names would freeze the UI mid-keystroke; doing it properly needs a cache and a
  hard timeout, which is its own piece of work.
- No `$PATH` executable completion for shell commands.
- No completion in the F7 mkdir / Shift+F4 new-file dialogs.
- No inactive-panel candidate source.
- No mouse support, no multi-column menu.
- Ghost text is literal-prefix, not fuzzy — see `history.rs` above.

## Error handling

Completion and history must never break the file manager, mirroring how logging
already treats failure:

- History read/write failures are swallowed; a missing or corrupt history file
  is treated as empty.
- Filesystem errors in path sources yield an empty candidate list, as today.
- An empty candidate set leaves the buffer untouched and opens no menu.

## Testing

- **`matcher.rs`** — pure unit tests: subsequence hits and misses, smartcase in
  both directions, exact-prefix outranking fuzzy, and a determinism test
  asserting a fixed candidate set always ranks in one exact order.
- **`history.rs`** — `TempDir` plus an `XDG_DATA_HOME` override. The load-bearing
  test asserts a `Shell` line is **never** written to disk while remaining
  available to `suggest` in the same session. Cap and dedup behaviour; corrupt
  file tolerated.
- **`sources.rs`** — `TempDir` panels; bookmark source offers only populated
  slots and carries the path as `hint`.
- **`popup.rs`** — the key matrix above, both menu states, including `Esc`
  closing only the menu.
- **`ui.rs`** — `TestBackend` render assertions for ghost text and for a menu
  longer than 10 rows.
