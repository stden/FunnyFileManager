<!-- Purpose: Phase 1 implementation plan for smart autocomplete (fuzzy matching + dropdown menu). -->

# Smart Autocomplete Phase 1 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace prefix-only `:` command-line completion with fuzzy-ranked
candidates shown in a navigable dropdown menu.

**Architecture:** A new pure `complete/matcher.rs` ranks candidates; the existing
`complete.rs` keeps its public entry point and delegates matching to it.
`Overlay::CommandLine` grows from a bare `String` into a `CommandLine` struct
carrying the buffer plus optional menu state, which `popup.rs` drives and
`ui.rs` renders.

**Tech Stack:** Rust 2021 (rust-version 1.77), `ratatui` 0.30, `crossterm` 0.29.
Tests are `#[cfg(test)]` child modules (`src/<module>/tests*.rs`), using
`tempfile::TempDir` for filesystem isolation and `ratatui::backend::TestBackend`
for render assertions.

## Global Constraints

- No file in `src/` may exceed 1000 lines (enforced by
  `tests/file_line_limit_test.rs`); split files over ~500 lines into modules.
- Unit tests live in `<module>/tests*.rs` child modules, declared `#[cfg(test)] mod tests;`.
- Std library before third-party crates. No new dependencies in this phase.
- `cargo fmt --all -- --check` and `cargo clippy --lib` must be clean
  (clippy runs with `pedantic` + `nursery`; keep first doc paragraphs short and
  backtick identifiers like `` `PgDn` `` in doc comments).
- Spec: `docs/superpowers/specs/2026-07-25-smart-autocomplete-design.md`.
- This repo is edited by a concurrent agent. Before each commit, run
  `git status --short`, stage **only** the files listed in that task, and verify
  with `git diff --cached --stat`. Never `git add -A`.

## File Structure

| File | Responsibility |
|---|---|
| `src/complete/matcher.rs` (create) | Pure fuzzy scoring + deterministic ranking. No `App`, no I/O. |
| `src/complete/matcher/tests.rs` (create) | Unit tests for scoring, smartcase, tiering, determinism. |
| `src/complete.rs` (modify) | Declares `pub mod matcher;`; `complete()` ranks through it. |
| `src/models.rs` (modify) | `CommandLine` + `Menu` + `Candidate` types; `Overlay::CommandLine(CommandLine)`. |
| `src/popup.rs` (modify) | Menu key handling in the command-line overlay. |
| `src/ui.rs` (modify) | Renders the dropdown above the command line. |

---

### Task 1: Fuzzy matcher (pure, no ripple)

Self-contained: creates one new module with no callers yet, so it cannot break
anything else.

**Files:**
- Create: `src/complete/matcher.rs`
- Create: `src/complete/matcher/tests.rs`
- Modify: `src/complete.rs` (add `pub mod matcher;` near the top, after the `use` block)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub enum Tier { Fuzzy, Prefix }` (derives `PartialOrd`, `Ord`)
  - `pub struct Score { pub tier: Tier, pub points: i32 }`
  - `pub fn score(query: &str, candidate: &str) -> Option<Score>`
  - `pub fn rank(query: &str, candidates: Vec<String>) -> Vec<String>`

- [ ] **Step 1: Write the failing tests**

Create `src/complete/matcher/tests.rs`:

```rust
//! Unit tests: fuzzy matching and ranking.
#![allow(unused_imports)]

use super::*;

#[test]
fn test_exact_prefix_scores_in_the_prefix_tier() {
    let score = score("cd", "cd").expect("prefix match");
    assert_eq!(score.tier, Tier::Prefix);
}

#[test]
fn test_subsequence_scores_in_the_fuzzy_tier() {
    // d-w-n appear in order inside "Downloads/", but not as a prefix.
    let score = score("dwn", "Downloads/").expect("fuzzy match");
    assert_eq!(score.tier, Tier::Fuzzy);
}

#[test]
fn test_non_subsequence_does_not_match() {
    assert_eq!(score("zzz", "Downloads/"), None);
}

#[test]
fn test_out_of_order_characters_do_not_match() {
    // "nwd" has the right letters in the wrong order.
    assert_eq!(score("nwd", "Downloads/"), None);
}

#[test]
fn test_empty_query_matches_everything_in_the_prefix_tier() {
    let score = score("", "anything").expect("empty query matches");
    assert_eq!(score.tier, Tier::Prefix);
}

#[test]
fn test_lowercase_query_is_case_insensitive() {
    assert!(score("dow", "Downloads/").is_some());
}

#[test]
fn test_uppercase_in_query_forces_case_sensitivity() {
    // Smartcase: any uppercase character makes the whole query case-sensitive.
    assert!(score("Dow", "Downloads/").is_some());
    assert_eq!(score("Dow", "downloads/"), None);
}

#[test]
fn test_consecutive_run_outscores_a_scattered_match() {
    let tight = score("cat", "cat.txt").expect("match");
    let loose = score("cat", "c_a_t.txt").expect("match");
    assert!(
        tight.points > loose.points,
        "tight={tight:?} should outscore loose={loose:?}"
    );
}

#[test]
fn test_rank_puts_prefix_matches_above_fuzzy_ones() {
    // "cd" is a prefix of "cd" and a subsequence of "copy_dir".
    let ranked = rank("cd", vec!["copy_dir".to_string(), "cd".to_string()]);
    assert_eq!(ranked, vec!["cd".to_string(), "copy_dir".to_string()]);
}

#[test]
fn test_rank_drops_non_matching_candidates() {
    let ranked = rank("cd", vec!["cd".to_string(), "zzz".to_string()]);
    assert_eq!(ranked, vec!["cd".to_string()]);
}

#[test]
fn test_rank_is_deterministic_for_equally_scored_candidates() {
    // All three are prefix matches of equal length, so the lexicographic
    // tiebreak decides — ranking must never vary between runs.
    let ranked = rank(
        "a",
        vec!["ac".to_string(), "aa".to_string(), "ab".to_string()],
    );
    assert_eq!(
        ranked,
        vec!["aa".to_string(), "ab".to_string(), "ac".to_string()]
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib matcher`
Expected: FAIL to compile — `file not found for module 'matcher'`, then once the
module is declared, `cannot find function 'score' in this scope`.

- [ ] **Step 3: Write the minimal implementation**

Create `src/complete/matcher.rs`:

```rust
//! Purpose: Fuzzy matching and deterministic ranking of completion candidates.
//!
//! Pure: no `App`, no filesystem, no I/O — everything here is a function of its
//! arguments, which is what makes the ranking cheap to test exhaustively.

/// Ranking tier. Exact-prefix matches always beat fuzzy ones, so typing `cd`
/// can never be outranked by a fuzzy hit on some longer verb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Fuzzy,
    Prefix,
}

/// How well a candidate matches a query; higher `points` is better.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Score {
    pub tier: Tier,
    pub points: i32,
}

/// Characters after which a match counts as starting a new word.
const WORD_BOUNDARIES: [char; 5] = ['/', '_', '-', '.', ' '];

/// Points awarded for a match continuing a run, opening a word, or neither.
const CONSECUTIVE_BONUS: i32 = 10;
const BOUNDARY_BONUS: i32 = 8;
const PLAIN_MATCH: i32 = 1;
/// Deducted per candidate character skipped over.
const GAP_PENALTY: i32 = 1;

/// Score `candidate` against `query`, or `None` when it does not match.
///
/// Matching is subsequence-based: every query character must appear in
/// `candidate`, in order. **Smartcase** — an all-lowercase query matches
/// case-insensitively; any uppercase character makes the query case-sensitive.
#[must_use]
pub fn score(query: &str, candidate: &str) -> Option<Score> {
    if query.is_empty() {
        return Some(Score {
            tier: Tier::Prefix,
            points: 0,
        });
    }

    let case_sensitive = query.chars().any(char::is_uppercase);
    let normalize = |c: char| {
        if case_sensitive {
            c
        } else {
            c.to_ascii_lowercase()
        }
    };

    let haystack: Vec<char> = candidate.chars().map(normalize).collect();
    let needle: Vec<char> = query.chars().map(normalize).collect();

    let mut points = 0;
    let mut haystack_index = 0;
    let mut previous_match: Option<usize> = None;

    for wanted in needle {
        let found = haystack[haystack_index..]
            .iter()
            .position(|&c| c == wanted)
            .map(|offset| haystack_index + offset)?;

        points += if previous_match == Some(found.wrapping_sub(1)) {
            CONSECUTIVE_BONUS
        } else if found == 0 || WORD_BOUNDARIES.contains(&haystack[found - 1]) {
            BOUNDARY_BONUS
        } else {
            PLAIN_MATCH
        };
        points -= GAP_PENALTY * i32::try_from(found - haystack_index).unwrap_or(0);

        previous_match = Some(found);
        haystack_index = found + 1;
    }

    let tier = if is_prefix(&needle_of(query, case_sensitive), candidate, case_sensitive) {
        Tier::Prefix
    } else {
        Tier::Fuzzy
    };

    Some(Score { tier, points })
}

/// The query as compared — lowercased unless the query is case-sensitive.
fn needle_of(query: &str, case_sensitive: bool) -> String {
    if case_sensitive {
        query.to_string()
    } else {
        query.to_ascii_lowercase()
    }
}

/// Whether `candidate` starts with `needle` under the active case rule.
fn is_prefix(needle: &str, candidate: &str, case_sensitive: bool) -> bool {
    if case_sensitive {
        candidate.starts_with(needle)
    } else {
        candidate.to_ascii_lowercase().starts_with(needle)
    }
}

/// Keep the candidates matching `query`, best first.
///
/// The ordering key is `(tier desc, points desc, length asc, lexicographic asc)`
/// — total and deterministic, because a ranking that varies between runs cannot
/// be tested.
#[must_use]
pub fn rank(query: &str, candidates: Vec<String>) -> Vec<String> {
    let mut scored: Vec<(Score, String)> = candidates
        .into_iter()
        .filter_map(|candidate| score(query, &candidate).map(|s| (s, candidate)))
        .collect();

    scored.sort_by(|(a_score, a), (b_score, b)| {
        b_score
            .tier
            .cmp(&a_score.tier)
            .then(b_score.points.cmp(&a_score.points))
            .then(a.len().cmp(&b.len()))
            .then(a.cmp(b))
    });

    scored.into_iter().map(|(_, candidate)| candidate).collect()
}

#[cfg(test)]
mod tests;
```

Add to `src/complete.rs`, immediately after its `use` statements:

```rust
pub mod matcher;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib matcher`
Expected: PASS, 11 tests.

Then: `cargo clippy --lib` — expected clean. If clippy flags
`too_long_first_doc_paragraph`, shorten the first paragraph and move detail into
a second one.

- [ ] **Step 5: Commit**

```bash
git status --short
git add src/complete/matcher.rs src/complete/matcher/tests.rs src/complete.rs
git diff --cached --stat
git commit -m "feat: fuzzy matcher for command-line completion

- [+] complete::matcher — subsequence scoring with consecutive/word-boundary
  bonuses, smartcase, and a Prefix tier that always outranks Fuzzy
- [+] rank() — total, deterministic ordering so results never vary between runs"
```

---

### Task 2: `CommandLine` state struct (mechanical, no behaviour change)

Pure refactor: the payload of `Overlay::CommandLine` becomes a struct holding a
single `buffer` field. No menu yet — that keeps this task reviewable as
"nothing should behave differently".

**Files:**
- Modify: `src/models.rs` (new struct + `Overlay::CommandLine` variant + two accessors)
- Modify: `src/lib.rs` (construction site in the `Action::CommandLine` arm)
- Modify: `src/popup.rs` (the `Overlay::CommandLine(s)` match arm)
- Modify: `src/ui.rs` (`render_command_line` payload read)
- Modify (test call sites): `src/tests.rs`, `src/tests_command_line_help.rs`,
  `src/models/tests.rs`, `src/ui/tests.rs`, `src/ui/tests_overlay_dialogs.rs`,
  `src/handlers/tests_popup_command_line.rs`,
  `src/handlers/tests_popup_input_keys.rs`,
  `src/handlers/tests_search_and_commands.rs`,
  `src/handlers/tests_source_paths_fallbacks.rs`,
  `src/handlers/tests_new_file_and_drives.rs`,
  `src/handlers/tests_copy_move_edge_cases.rs`

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces:
  - `pub struct CommandLine { pub buffer: String }`
  - `impl From<&str> for CommandLine`, `impl From<String> for CommandLine`
  - `Overlay::CommandLine(CommandLine)`
  - `App::get_command_line(&self) -> Option<&str>` — **signature unchanged**
  - `App::get_command_line_mut(&mut self) -> Option<&mut String>` — **signature unchanged**

- [ ] **Step 1: Write the failing test**

Append to `src/models/tests.rs`:

```rust
#[test]
fn test_command_line_overlay_exposes_its_buffer() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("cd /tmp".into()));

    // The accessors keep their old shape so existing call sites are unaffected.
    assert_eq!(app.get_command_line(), Some("cd /tmp"));

    app.get_command_line_mut().expect("command line").push('/');
    assert_eq!(app.get_command_line(), Some("cd /tmp/"));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib test_command_line_overlay_exposes_its_buffer`
Expected: FAIL to compile — `expected 'String', found 'CommandLine'` at the
`.into()` call, because `Overlay::CommandLine` still holds a `String`.

- [ ] **Step 3: Write the minimal implementation**

In `src/models.rs`, add the struct next to the other overlay payload types:

```rust
/// State of the `:` command line: the text typed so far.
///
/// A struct rather than a bare `String` because completion needs to hang menu
/// state off the same overlay in Task 4 without another sweeping change.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandLine {
    pub buffer: String,
}

impl From<String> for CommandLine {
    fn from(buffer: String) -> Self {
        Self { buffer }
    }
}

impl From<&str> for CommandLine {
    fn from(buffer: &str) -> Self {
        Self {
            buffer: buffer.to_owned(),
        }
    }
}
```

Change the enum variant:

```rust
    /// `:` command line
    CommandLine(CommandLine),
```

Update the two accessors to reach through the struct — note the **return types
do not change**, which is what spares the majority of call sites:

```rust
    pub fn get_command_line(&self) -> Option<&str> {
        if let Overlay::CommandLine(state) = &self.overlay {
            Some(&state.buffer)
        } else {
            None
        }
    }

    pub fn get_command_line_mut(&mut self) -> Option<&mut String> {
        if let Overlay::CommandLine(state) = &mut self.overlay {
            Some(&mut state.buffer)
        } else {
            None
        }
    }
```

In `src/lib.rs`, the `Action::CommandLine` arm:

```rust
        Action::CommandLine => app.set_overlay(Overlay::CommandLine(CommandLine::default())),
```

(add `CommandLine` to the existing `use crate::models::{App, Overlay};` import).

In `src/popup.rs`, the match arm binds the struct — rename the binding so the
body reads clearly, then use `state.buffer` wherever `s` was used:

```rust
        Overlay::CommandLine(state) => {
            match code {
                KeyCode::Esc => {
                    app.close_overlay();
                    app.quick_search = None;
                    true
                }
                KeyCode::Enter => {
                    let command = state.buffer.trim().to_string();
                    app.close_overlay();
                    if !command.is_empty() {
                        if let Err(err) = crate::handlers::execute_command(app, &command) {
                            app.set_status(crate::status::Status::Error(err.to_string()));
                        }
                    }
                    true
                }
                KeyCode::Backspace => {
                    state.buffer.pop();
                    true
                }
                KeyCode::Tab => {
                    // Clone the buffer so the `&mut app.overlay` borrow ends
                    // before we touch the panel / status.
                    let current = state.buffer.clone();
                    let cwd = app.get_active_panel().path.clone();
                    if let Some(comp) = crate::complete::complete(&current, &cwd) {
                        if let Overlay::CommandLine(state) = &mut app.overlay {
                            state.buffer = comp.line;
                        }
                        if comp.candidates.len() > 1 {
                            app.set_status(crate::status::Status::Completions {
                                options: comp.candidates.join("  "),
                            });
                        }
                    }
                    true
                }
                KeyCode::Char(ch)
                    if !modifiers.contains(KeyModifiers::CONTROL)
                        && !modifiers.contains(KeyModifiers::ALT) =>
                {
                    state.buffer.push(ch);
                    true
                }
                _ => false,
            }
        }
```

In `src/ui.rs`, `render_command_line`:

```rust
    let cmd_text = if let Overlay::CommandLine(ref state) = app.overlay {
        state.buffer.as_str()
    } else {
        ""
    };
```

For the test files, apply these two mechanical rewrites:

```
Overlay::CommandLine(String::new())      ->  Overlay::CommandLine(CommandLine::default())
Overlay::CommandLine("TEXT".to_string()) ->  Overlay::CommandLine("TEXT".into())
if let Overlay::CommandLine(ref s) = ...  ->  if let Overlay::CommandLine(ref s) = ...
                                              then read `s.buffer` instead of `s`
```

`matches!(app.overlay, Overlay::CommandLine(_))` needs no change. Add
`CommandLine` to each test file's `use crate::models::{...}` import where the
compiler asks for it.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS — the same count as before this task plus 1. Any failure here is
a missed call site, not a behaviour change.

Then: `cargo fmt --all` and `cargo clippy --lib` — expected clean.

- [ ] **Step 5: Commit**

```bash
git status --short
git add src/models.rs src/lib.rs src/popup.rs src/ui.rs src/models/tests.rs \
        src/tests.rs src/tests_command_line_help.rs src/ui/tests.rs \
        src/ui/tests_overlay_dialogs.rs src/handlers/tests_popup_command_line.rs \
        src/handlers/tests_popup_input_keys.rs src/handlers/tests_search_and_commands.rs \
        src/handlers/tests_source_paths_fallbacks.rs \
        src/handlers/tests_new_file_and_drives.rs \
        src/handlers/tests_copy_move_edge_cases.rs
git diff --cached --stat
git commit -m "refactor: Overlay::CommandLine carries a CommandLine struct

- [!] models::CommandLine — the \`:\` overlay payload becomes a struct so
  completion can hang menu state off it; accessors keep their signatures
- no behaviour change"
```

---

### Task 3: Rank `complete()` results through the matcher

**Files:**
- Modify: `src/complete.rs` (`prefix_filter` and `path_complete` results go through `matcher::rank`)
- Modify: `src/complete/tests.rs` (add the fuzzy-behaviour test)

**Interfaces:**
- Consumes: `matcher::rank` from Task 1.
- Produces: `complete(input: &str, cwd: &Path) -> Option<Completion>` —
  **signature unchanged**; `Completion.candidates` is now ranked best-first and
  includes fuzzy matches.

- [ ] **Step 1: Write the failing test**

Append to `src/complete/tests.rs`:

```rust
#[test]
fn test_verb_completion_accepts_a_fuzzy_query() {
    // "bkm" is not a prefix of "bookmark", but it is a subsequence.
    let completion = complete("bkm", Path::new(".")).expect("fuzzy verb match");
    assert!(
        completion.candidates.contains(&"bookmark".to_string()),
        "candidates were {:?}",
        completion.candidates
    );
}

#[test]
fn test_exact_prefix_candidate_ranks_first() {
    // "m" prefixes both "mark" and "mcp"; a fuzzy hit must not displace them.
    let completion = complete("m", Path::new(".")).expect("prefix match");
    assert_eq!(
        completion.candidates.first(),
        Some(&"mcp".to_string()),
        "candidates were {:?}",
        completion.candidates
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib complete::tests::test_verb_completion_accepts_a_fuzzy_query`
Expected: FAIL — `complete` returns `None`, so `.expect("fuzzy verb match")`
panics, because `prefix_filter` only keeps `starts_with` hits.

- [ ] **Step 3: Write the minimal implementation**

In `src/complete.rs`, replace the body of `prefix_filter` and rename it, since it
is no longer prefix-only:

```rust
/// Keep the options matching `token`, best first (see [`matcher::rank`]).
fn match_filter(options: &[&str], token: &str) -> Vec<String> {
    matcher::rank(
        token,
        options.iter().map(|&o| o.to_string()).collect(),
    )
}
```

Update every `prefix_filter(` call site in `complete()` to `match_filter(`.

In `path_complete`, rank the results before returning:

```rust
fn path_complete(token: &str, cwd: &Path) -> Vec<String> {
    let candidates = fish_path_complete(token, cwd)
        .unwrap_or_else(|| builtin_path_complete(token, cwd));
    // Rank against the leaf only: the directory part is already typed and
    // shared by every candidate, so scoring it adds nothing.
    let leaf = token.rsplit('/').next().unwrap_or(token);
    matcher::rank(leaf, candidates)
}
```

In `builtin_path_complete`, replace the `if !name.starts_with(leaf) { return None; }`
guard with a match test so fuzzy leaves survive to the ranking step:

```rust
            if matcher::score(leaf, &name).is_none() {
                return None;
            }
```

and drop the now-redundant `out.sort()` at the end of that function — `rank`
imposes the order.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib complete`
Expected: PASS. Some existing `complete` tests may assert candidate ordering; if
one fails, confirm the new order is correct per the spec's ranking rule and
update that assertion, noting why in the commit message.

- [ ] **Step 5: Commit**

```bash
git status --short
git add src/complete.rs src/complete/tests.rs
git diff --cached --stat
git commit -m "feat: rank command-line completion fuzzily

- [!] complete::complete — candidates are fuzzy-matched and ranked best-first
  instead of filtered by prefix; exact prefixes still rank above fuzzy hits"
```

---

### Task 4: Menu state and key handling

**Files:**
- Modify: `src/models.rs` (`Menu`, `Candidate`, `menu` field on `CommandLine`)
- Modify: `src/popup.rs` (Tab / Up / Down / Enter / Esc behaviour)
- Modify: `src/status.rs` (remove the now-redundant `Completions` variant)
- Modify: `src/handlers/tests_popup_command_line.rs` (key-matrix tests)

**Interfaces:**
- Consumes: `CommandLine` (Task 2), `complete()` (Task 3).
- Produces:
  - `pub struct Candidate { pub text: String, pub hint: Option<String> }`
  - `pub struct Menu { pub candidates: Vec<Candidate>, pub selected: usize }`
  - `Menu::next(&mut self)`, `Menu::previous(&mut self)` — wrap around
  - `CommandLine { pub buffer: String, pub menu: Option<Menu> }`

- [ ] **Step 1: Write the failing tests**

Append to `src/handlers/tests_popup_command_line.rs`:

```rust
#[test]
fn test_tab_opens_a_menu_when_several_candidates_match() {
    let mut app = App::new().expect("test setup");
    // "m" matches at least "mark", "mcp" and "unmark".
    app.set_overlay(Overlay::CommandLine("m".into()));

    process_popup_input(&mut app, key(KeyCode::Tab));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    let menu = state.menu.as_ref().expect("menu should be open");
    assert!(menu.candidates.len() > 1);
    assert_eq!(menu.selected, 0);
}

#[test]
fn test_tab_accepts_directly_when_only_one_candidate_matches() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("unm".into()));

    process_popup_input(&mut app, key(KeyCode::Tab));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(state.buffer, "unmark ");
    assert!(state.menu.is_none(), "a lone candidate opens no menu");
}

#[test]
fn test_down_moves_the_menu_selection() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key(KeyCode::Tab));

    process_popup_input(&mut app, key(KeyCode::Down));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(state.menu.as_ref().expect("menu").selected, 1);
}

#[test]
fn test_enter_accepts_the_selected_candidate_instead_of_running_it() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key(KeyCode::Tab));

    process_popup_input(&mut app, key(KeyCode::Enter));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("Enter with a menu open must accept, not execute and close");
    };
    assert!(state.menu.is_none(), "accepting closes the menu");
    assert!(!state.buffer.is_empty());
}

#[test]
fn test_esc_closes_only_the_menu_not_the_overlay() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key(KeyCode::Tab));

    process_popup_input(&mut app, key(KeyCode::Esc));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("first Esc must not close the command line");
    };
    assert!(state.menu.is_none());

    // A second Esc closes the overlay itself.
    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn test_typing_refreshes_an_open_menu() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key(KeyCode::Tab));

    process_popup_input(&mut app, key(KeyCode::Char('c')));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(state.buffer, "mc");
    let menu = state.menu.as_ref().expect("menu stays open while typing");
    assert_eq!(menu.selected, 0, "refreshing resets the selection");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib tests_popup_command_line`
Expected: FAIL to compile — `no field 'menu' on type 'CommandLine'`.

- [ ] **Step 3: Write the minimal implementation**

In `src/models.rs`:

```rust
/// One row of the completion dropdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Replacement text for the token being completed.
    pub text: String,
    /// Extra detail shown right-aligned — e.g. the path behind a bookmark slot.
    pub hint: Option<String>,
}

/// Open completion dropdown: the candidates and which one is highlighted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu {
    pub candidates: Vec<Candidate>,
    pub selected: usize,
}

impl Menu {
    /// Highlight the next candidate, wrapping at the end.
    pub fn next(&mut self) {
        if self.candidates.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.candidates.len();
    }

    /// Highlight the previous candidate, wrapping at the start.
    pub fn previous(&mut self) {
        if self.candidates.is_empty() {
            return;
        }
        self.selected = self
            .selected
            .checked_sub(1)
            .unwrap_or(self.candidates.len() - 1);
    }
}
```

Extend `CommandLine`:

```rust
pub struct CommandLine {
    pub buffer: String,
    /// `None` while the dropdown is closed.
    pub menu: Option<Menu>,
}
```

Both `From` impls set `menu: None`.

In `src/popup.rs`, extend the import to
`use crate::models::{App, BookmarkSlot, Candidate, Menu, Overlay, PanelSide};`,
then rewrite the `Overlay::CommandLine` arm's key handling. Place the two
guarded arms (`Down if …`, `Up if …`, `Enter if …`) **before** the existing
unguarded `Enter` arm:

```rust
                KeyCode::Tab => {
                    let current = state.buffer.clone();
                    let cwd = app.get_active_panel().path.clone();
                    let Some(comp) = crate::complete::complete(&current, &cwd) else {
                        return true;
                    };
                    if let Overlay::CommandLine(state) = &mut app.overlay {
                        state.buffer = comp.line;
                        // A lone candidate is accepted outright; the menu is for
                        // choosing between genuine alternatives.
                        state.menu = (comp.candidates.len() > 1).then(|| Menu {
                            candidates: comp
                                .candidates
                                .into_iter()
                                .map(|text| Candidate { text, hint: None })
                                .collect(),
                            selected: 0,
                        });
                    }
                    true
                }
                KeyCode::Down if state.menu.is_some() => {
                    state.menu.as_mut().expect("menu is some").next();
                    true
                }
                KeyCode::Up if state.menu.is_some() => {
                    state.menu.as_mut().expect("menu is some").previous();
                    true
                }
```

`Esc` closes the menu first when one is open. **`state` borrows `app`, so the
borrow must end before `app.close_overlay()`** — `take()` both clears the menu
and yields an owned value, ending the borrow on that line:

```rust
                KeyCode::Esc => {
                    // `take()` ends the `state` borrow before `app` is touched.
                    let had_menu = state.menu.take().is_some();
                    if !had_menu {
                        app.close_overlay();
                        app.quick_search = None;
                    }
                    true
                }
```

`Enter` accepts the highlighted candidate when the menu is open. **This arm must
sit above the existing plain `KeyCode::Enter` arm** — match arms are tried in
order, and the guard is what routes between accepting and executing:

```rust
                KeyCode::Enter if state.menu.is_some() => {
                    let menu = state.menu.take().expect("menu is some");
                    if let Some(candidate) = menu.candidates.get(menu.selected) {
                        // Replace the last token with the chosen candidate.
                        let head = state
                            .buffer
                            .rfind(char::is_whitespace)
                            .map_or("", |i| &state.buffer[..=i])
                            .to_string();
                        state.buffer = format!("{head}{}", candidate.text);
                    }
                    true
                }
```

Typing refreshes an open menu — after `state.buffer.push(ch)` in the
`KeyCode::Char(ch)` arm, reset the selection so the highlight never points past
the new candidate list:

```rust
                KeyCode::Char(ch)
                    if !modifiers.contains(KeyModifiers::CONTROL)
                        && !modifiers.contains(KeyModifiers::ALT) =>
                {
                    state.buffer.push(ch);
                    if let Some(menu) = state.menu.as_mut() {
                        menu.selected = 0;
                    }
                    true
                }
```

Remove the `app.set_status(Status::Completions { .. })` call — the menu shows
the candidates now. Then delete the variant from `src/status.rs` (the
declaration near `Completions { options: String }`, its `Display` arm, and its
row in the status test table).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS, including the six new tests.

Then: `cargo fmt --all` and `cargo clippy --lib` — expected clean.

- [ ] **Step 5: Commit**

```bash
git status --short
git add src/models.rs src/popup.rs src/status.rs src/handlers/tests_popup_command_line.rs
git diff --cached --stat
git commit -m "feat: dropdown menu for command-line completion

- [+] models::Menu / Candidate — completion candidates with a highlighted row
- [!] process_popup_input — Tab opens the menu (a lone candidate is accepted
  outright), Up/Down move, Enter accepts, Esc closes the menu before the overlay
- [-] Status::Completions — the menu replaces the status-bar candidate list"
```

---

### Task 5: Render the dropdown

**Files:**
- Modify: `src/ui.rs` (`render_command_line` draws the menu above the command line)
- Modify: `src/ui/tests.rs` (render assertions)

**Interfaces:**
- Consumes: `Menu`, `Candidate` (Task 4).
- Produces: no new public API; `MENU_MAX_ROWS` is private to `ui.rs`.

- [ ] **Step 1: Write the failing tests**

Extend the import in `src/ui/tests.rs` to
`use crate::models::{App, Candidate, CommandLine, DirEntry, Menu, Overlay};`,
then append:

```rust
#[test]
fn test_open_menu_renders_its_candidates() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::CommandLine(CommandLine {
        buffer: "m".to_string(),
        menu: Some(Menu {
            candidates: vec![
                Candidate { text: "mark".to_string(), hint: None },
                Candidate { text: "mcp".to_string(), hint: None },
            ],
            selected: 0,
        }),
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(screen.contains("mark"), "{screen}");
    assert!(screen.contains("mcp"), "{screen}");
}

#[test]
fn test_menu_renders_at_most_ten_rows() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    let candidates: Vec<Candidate> = (0..30)
        .map(|i| Candidate {
            text: format!("candidate{i:02}"),
            hint: None,
        })
        .collect();
    app.set_overlay(Overlay::CommandLine(CommandLine {
        buffer: "c".to_string(),
        // Selection near the end must stay visible, so the window scrolls.
        menu: Some(Menu { candidates, selected: 29 }),
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(screen.contains("candidate29"), "selection must be visible: {screen}");
    assert!(!screen.contains("candidate00"), "window should have scrolled: {screen}");
}

#[test]
fn test_candidate_hint_is_rendered() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::CommandLine(CommandLine {
        buffer: "bookmark go ".to_string(),
        menu: Some(Menu {
            candidates: vec![Candidate {
                text: "3".to_string(),
                hint: Some("/srv/FunnyFileManager".to_string()),
            }],
            selected: 0,
        }),
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(screen.contains("/srv/FunnyFileManager"), "{screen}");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib ui::tests::test_open_menu_renders_its_candidates`
Expected: FAIL — the assertion panics because nothing draws the menu yet
(`screen` contains the panels and command line but no candidate text).

- [ ] **Step 3: Write the minimal implementation**

In `src/ui.rs`, add the constant next to the other layout constants:

```rust
/// Most candidate rows the completion dropdown shows at once.
const MENU_MAX_ROWS: usize = 10;
```

Add the renderer, and call it from `ui()` right after `render_command_line`:

```rust
/// Draw the completion dropdown directly above the command line.
///
/// The window scrolls to keep the highlighted candidate visible, so a long
/// candidate list stays navigable without growing the popup.
fn render_completion_menu(f: &mut Frame, app: &App, command_line_area: Rect) {
    let Overlay::CommandLine(state) = &app.overlay else {
        return;
    };
    let Some(menu) = &state.menu else {
        return;
    };
    if menu.candidates.is_empty() {
        return;
    }

    let visible = menu.candidates.len().min(MENU_MAX_ROWS);
    // Scroll so `selected` sits inside the window.
    let first = menu.selected.saturating_sub(visible - 1);

    let height = u16::try_from(visible).unwrap_or(u16::MAX) + PANEL_BORDERS;
    let area = Rect {
        x: command_line_area.x,
        y: command_line_area.y.saturating_sub(height),
        width: command_line_area.width,
        height: height.min(command_line_area.y),
    };

    let inner_width = usize::from(area.width.saturating_sub(PANEL_BORDERS));
    let items: Vec<ListItem> = menu
        .candidates
        .iter()
        .enumerate()
        .skip(first)
        .take(visible)
        .map(|(index, candidate)| {
            let hint = candidate.hint.as_deref().unwrap_or("");
            let gap = inner_width
                .saturating_sub(candidate.text.chars().count() + hint.chars().count());
            let style = if index == menu.selected {
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };
            ListItem::new(Line::from(Span::styled(
                format!("{}{}{hint}", candidate.text, " ".repeat(gap)),
                style,
            )))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(FAR_BLUE)),
    );

    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(list, area);
}
```

In `ui()`:

```rust
    render_command_line(f, app, chunks[3]);
    render_completion_menu(f, app, chunks[3]);
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib ui`
Expected: PASS, including the three new tests.

Then run the whole gate:
`cargo fmt --all -- --check && cargo clippy --lib && cargo test`
Expected: clean, all green.

- [ ] **Step 5: Commit**

```bash
git status --short
git add src/ui.rs src/ui/tests.rs
git diff --cached --stat
git commit -m "feat: render the completion dropdown

- [+] ui::render_completion_menu — candidate list above the command line, at
  most 10 rows, scrolling to keep the selection visible, hints right-aligned"
```

---

## Verification

After Task 5, phase 1 is complete when all of these hold:

```bash
cargo fmt --all -- --check   # clean
cargo clippy --lib           # no warnings
cargo test                   # all targets green
```

Manual check in a real terminal (the part no test covers — the harness cannot
send keystrokes to a pty):

1. `ffm`, press `:`, type `m`, press `Tab` → dropdown lists `mark`, `mcp`, … .
2. `Down` `Down` → highlight moves; `Enter` → that candidate lands in the buffer.
3. `Tab` again, then `Esc` → menu closes, typed text survives; `Esc` → overlay closes.
4. Type `cd dwn` and `Tab` in a directory containing `Downloads` → fuzzy hit.

## Not in this phase

History persistence, ghost text, panel-entry and bookmark candidate sources —
those are phases 2 and 3 of the spec.
