<!-- Purpose: Phase 3 implementation plan for smart autocomplete (bookmark-slot candidate source). -->

# Smart Autocomplete Phase 3 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** `bookmark go`/`bookmark set` completion offers only the slots that
actually hold a path, each showing that path as a dropdown hint — instead of
today's blind list of digits 0–9.

**Architecture:** Completion candidates become the same `Candidate` type the
dropdown already renders (text + kind + hint), so a per-candidate hint travels
with its text end to end. `complete()` gains a `CompletionContext` carrying the
active panel's `cwd` and the bookmark slots; `popup.rs` builds it from `App`.

**Scope note (approved decision):** the spec listed a second context source,
*active panel entries*, for `cd`. It is **dropped**: `complete()` already scans
the active panel's directory for `cd`, so panel entries would duplicate that
scan (they are the same directory's contents) — a KISS violation with no
user-visible gain. `cd` has the only path argument in the command DSL, so panel
entries have no other home. The spec is updated to record this in Task 3.

**Tech Stack:** Rust 2021 (rust-version 1.95), `ratatui` 0.30. No new deps.

## Global Constraints

- No file in `src/` over 1000 lines; unit tests in `<module>/tests*.rs`.
- `cargo fmt --all -- --check` and `cargo clippy --lib` clean (pedantic +
  nursery: short first doc paragraph, backticked identifiers in docs).
- TDD every task: failing test first, watch it fail, minimal code, green.
- Concurrent agent edits this repo: stage only each task's named files, never
  `git add -A`; `git status --short` before staging, `git diff --cached --stat`
  after; push after each task.
- `Candidate` (models.rs) is `{ text: String, kind: CandidateKind, hint:
  Option<String> }` and the dropdown renderer (`ui.rs::render_completion_menu`)
  already displays `hint` right-aligned — no rendering change is needed.

## File Structure

| File | Responsibility |
|---|---|
| `src/complete.rs` (modify) | `Completion.candidates: Vec<Candidate>`; `CompletionContext`; bookmark-slot source |
| `src/complete/tests.rs` (modify) | update `.candidates` asserts to `Candidate`; bookmark-source tests |
| `src/popup.rs` (modify) | build `CompletionContext` from `App`; `open_menu` takes `Vec<Candidate>` |
| `src/models.rs` (modify only if needed) | `Candidate` may need `Clone`/`PartialEq` for tests |
| `src/ops/dispatch.rs` (modify) | one test call site of `complete()` |
| `docs/superpowers/specs/2026-07-25-smart-autocomplete-design.md` (modify) | record the panel-entries drop |

---

### Task 1: `Completion` carries `Vec<Candidate>` and a `CompletionContext` (pure refactor)

No behaviour change: completion offers the same candidates as today, just
typed as `Candidate` (all `hint: None`) and reached through a context whose
only field used so far is `cwd`. This isolates the wide type change from the
feature so the review gate for each is clean.

**Files:**
- Modify: `src/complete.rs`
- Modify: `src/complete/tests.rs`
- Modify: `src/popup.rs`
- Modify: `src/ops/dispatch.rs` (test call site at ~line 473)
- Modify: `src/models.rs` **only if** `Candidate` lacks derives the tests need

**Interfaces:**
- Produces:
  - `pub struct CompletionContext<'a> { pub cwd: &'a std::path::Path, pub bookmarks: &'a [Option<std::path::PathBuf>] }`
  - `impl<'a> CompletionContext<'a> { pub fn for_cwd(cwd: &'a Path) -> Self }` — `bookmarks: &[]`
  - `pub fn complete(input: &str, cwd: &Path) -> Option<Completion>` — **kept** as a thin wrapper: `complete_with(input, &CompletionContext::for_cwd(cwd))`. Keeps every existing bookmark-free call site (33 in tests, 1 in ops/dispatch) compiling unchanged.
  - `pub fn complete_with(input: &str, ctx: &CompletionContext) -> Option<Completion>` — the real entry.
  - `Completion { pub line: String, pub candidates: Vec<Candidate> }` — the top-level `kind` field is **removed**; each `Candidate` carries its own `kind`.

- [ ] **Step 1: Write/adjust the failing tests**

The type change makes existing `.candidates` assertions fail to compile
(`Vec<String>` vs `Vec<Candidate>`). Add a test helper at the top of
`src/complete/tests.rs` and use it to keep assertions readable:

```rust
/// The `text` of each ranked candidate, for asserting on completion output.
fn texts(c: &Completion) -> Vec<String> {
    c.candidates.iter().map(|cand| cand.text.clone()).collect()
}
```

Then mechanically update assertions from
`assert_eq!(c.candidates, vec!["pwd"])` to
`assert_eq!(texts(&c), vec!["pwd".to_string()])`, and
`c.candidates.contains(&"x".to_string())` to
`texts(&c).contains(&"x".to_string())`, and any `c.candidates.len()` stays as
`c.candidates.len()`. Add one new test pinning the wrapper/ctx equivalence:

```rust
#[test]
fn complete_and_complete_with_for_cwd_agree() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("Documents")).unwrap();
    let a = complete("cd Doc", tmp.path()).expect("some");
    let b = complete_with("cd Doc", &CompletionContext::for_cwd(tmp.path())).expect("some");
    assert_eq!(a.line, b.line);
    assert_eq!(texts(&a), texts(&b));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib complete`
Expected: FAIL to compile — `Completion` has no field `kind` / `candidates` is
`Vec<Candidate>` not `Vec<String>` / `complete_with` not found. That compile
failure IS the red for a pure-refactor task.

- [ ] **Step 3: Implement**

In `src/complete.rs`:

```rust
use crate::models::{Candidate, CandidateKind};

/// Sources a completion draws on beyond the raw input. `cwd` is the active
/// panel's directory; `bookmarks` are its slots (index = slot number, `None` =
/// empty). Phase 3 adds `bookmarks`; earlier phases used only `cwd`.
pub struct CompletionContext<'a> {
    pub cwd: &'a std::path::Path,
    pub bookmarks: &'a [Option<std::path::PathBuf>],
}

impl<'a> CompletionContext<'a> {
    /// Context with no bookmark slots — for positions that never complete a
    /// bookmark, and for tests that do not exercise bookmarks.
    #[must_use]
    pub fn for_cwd(cwd: &'a std::path::Path) -> Self {
        Self { cwd, bookmarks: &[] }
    }
}

/// Outcome of a completion attempt.
pub struct Completion {
    /// The full replacement text for the command line.
    pub line: String,
    /// Ranked candidates, best first (len > 1 means the result was ambiguous
    /// and `line` advanced only to the longest common prefix).
    pub candidates: Vec<Candidate>,
}

#[must_use]
pub fn complete(input: &str, cwd: &std::path::Path) -> Option<Completion> {
    complete_with(input, &CompletionContext::for_cwd(cwd))
}

#[must_use]
pub fn complete_with(input: &str, ctx: &CompletionContext) -> Option<Completion> {
    // body moved from the old `complete`, using ctx.cwd wherever cwd was used.
    // ...
}
```

Convert each source to build `Candidate`s. The existing `match_filter` and
`path_complete` return `Vec<String>`; wrap their results into `Candidate`s with
the arm's kind and `hint: None`. Introduce one helper:

```rust
/// Wrap ranked plain strings as word/path candidates without hints.
fn plain_candidates(texts: Vec<String>, kind: CandidateKind) -> Vec<Candidate> {
    texts
        .into_iter()
        .map(|text| Candidate { text, kind, hint: None })
        .collect()
}
```

Each `match` arm now yields `Vec<Candidate>` directly (verbs/sub-commands via
`plain_candidates(match_filter(...), CandidateKind::Word)`, `cd` via
`plain_candidates(path_complete(...), CandidateKind::Path)`). The bookmark
sub-command and slot arms stay `plain_candidates(..., Word)` for now — the slot
source with hints lands in Task 2.

The accept/LCP logic that used the single `kind` now reads the candidate's own
kind. For the lone-candidate accept: `finish_word(&cands[0].text, cands[0].kind)`.
For the LCP path, compute the common prefix over the candidate *texts*:
`longest_common_prefix(&cands.iter().map(|c| c.text.clone()).collect::<Vec<_>>())`
(or adapt `longest_common_prefix` to take `&[Candidate]` and read `.text`).

In `src/popup.rs`:
- `open_menu` takes the candidates directly:
  ```rust
  fn open_menu(candidates: Vec<Candidate>) -> Menu {
      Menu { candidates, selected: 0 }
  }
  ```
  Update its two callers (Tab arm ~line 115, `refresh_menu` ~line 322):
  `comp.candidates.len() > 1` still gates opening; pass `comp.candidates`.
  The Tab lone-candidate accept path that used `comp.kind`/`comp.line` now uses
  `comp.line` (unchanged) — `comp.line` already reflects the finished single
  candidate, so the menu-vs-accept branch is unchanged apart from `open_menu`'s
  signature.
- Add the `Candidate` import if not already present.

In `src/ops/dispatch.rs` (~line 473): the test reads `completion.candidates`;
update it to read `.candidates[0].text` or use the same `texts`-style access,
matching whatever it asserts.

If the tests need `Candidate` to derive `Clone`/`PartialEq`/`Debug` and it does
not already, add the derives in `src/models.rs` (it likely already has them from
phase 1 — check first; only add what is missing).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib` then `cargo fmt --all` and `cargo clippy --lib`.
Expected: PASS, same count as baseline + 1 (the equivalence test). Zero
behaviour change — any candidate-order or `.line` test that changes means a bug
in the refactor, not a test to update.

- [ ] **Step 5: Commit**

```bash
git status --short
git add src/complete.rs src/complete/tests.rs src/popup.rs src/ops/dispatch.rs
# add src/models.rs only if you changed derives
git diff --cached --stat
git commit -m "refactor: completion yields Vec<Candidate> via a CompletionContext

- [!] Completion.candidates is now Vec<Candidate> (text+kind+hint) end to end,
  removing open_menu's String->Candidate conversion and the single top-level
  kind; each candidate carries its own kind
- [!] complete(input, cwd) kept as a thin wrapper over complete_with(input,
  &CompletionContext) so bookmark-free call sites are unchanged
- no behaviour change"
git push
```

---

### Task 2: Bookmark-slot source — populated slots with path hints

**Files:**
- Modify: `src/complete.rs` (bookmark slot arm reads `ctx.bookmarks`)
- Modify: `src/complete/tests.rs` (bookmark-source tests)
- Modify: `src/popup.rs` (build `CompletionContext` with `app.bookmarks`)

**Interfaces:**
- Consumes: `CompletionContext.bookmarks`, `Candidate` (Task 1).
- Produces: a `bookmark_slot_candidates(bookmarks, token) -> Vec<Candidate>`
  helper in `complete.rs`.

- [ ] **Step 1: Write the failing tests**

Append to `src/complete/tests.rs`:

```rust
use std::path::PathBuf;

fn ctx_with_bookmarks<'a>(
    cwd: &'a std::path::Path,
    slots: &'a [Option<PathBuf>],
) -> CompletionContext<'a> {
    CompletionContext { cwd, bookmarks: slots }
}

#[test]
fn bookmark_go_offers_only_populated_slots_with_path_hints() {
    let tmp = empty_cwd();
    let mut slots = vec![None; 10];
    slots[0] = Some(PathBuf::from("/srv/app"));
    slots[3] = Some(PathBuf::from("/home/me/dl"));

    let c = complete_with("bookmark go ", &ctx_with_bookmarks(tmp.path(), &slots))
        .expect("populated slots complete");
    assert_eq!(texts(&c), vec!["0".to_string(), "3".to_string()]);
    // The path rides along as the dropdown hint.
    let hint0 = c.candidates.iter().find(|c| c.text == "0").unwrap().hint.as_deref();
    let hint3 = c.candidates.iter().find(|c| c.text == "3").unwrap().hint.as_deref();
    assert_eq!(hint0, Some("/srv/app"));
    assert_eq!(hint3, Some("/home/me/dl"));
}

#[test]
fn bookmark_go_with_no_populated_slots_completes_nothing() {
    let tmp = empty_cwd();
    let slots = vec![None; 10];
    assert!(
        complete_with("bookmark go ", &ctx_with_bookmarks(tmp.path(), &slots)).is_none(),
        "no populated slot means nothing to offer"
    );
}

#[test]
fn bookmark_go_filters_populated_slots_by_typed_digit() {
    let tmp = empty_cwd();
    let mut slots = vec![None; 10];
    slots[3] = Some(PathBuf::from("/a"));
    slots[7] = Some(PathBuf::from("/b"));

    let c = complete_with("bookmark go 3", &ctx_with_bookmarks(tmp.path(), &slots))
        .expect("slot 3 matches");
    assert_eq!(texts(&c), vec!["3".to_string()]);
    assert_eq!(c.line, "bookmark go 3 ");
}

#[test]
fn bookmark_go_without_context_bookmarks_offers_nothing() {
    // The bare `complete()` wrapper has no bookmark slots, so `go` cannot list
    // any — it must not fall back to the blind 0-9 digit list.
    let tmp = empty_cwd();
    assert!(complete("bookmark go ", tmp.path()).is_none());
}
```

Note the last test pins a behaviour change: `bookmark go` completion now
depends on real slots, so the context-free wrapper offers nothing rather than
digits 0–9. Update any pre-existing test that asserted the old digit behaviour
for `bookmark go` (e.g. `bookmark_slot_completes_digit` may need to move to
`complete_with` + slots, or assert the new no-context behaviour) — verify
against the ranking rule and note the change in the commit.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib bookmark`
Expected: FAIL — the slot arm still returns blind digits, so hints are `None`
and populated-only filtering is absent.

- [ ] **Step 3: Implement**

In `src/complete.rs`, the `bookmark ... <slot>` arm (currently
`match_filter(&["0".."9"], token)`) becomes bookmark-aware. Keep `set` on the
plain digit list (you set into any slot, empty or not); make `go` list only
populated slots with hints:

```rust
Some(&"bookmark") if head_words.len() == 2 => {
    match head_words.get(1) {
        Some(&"go") => bookmark_slot_candidates(ctx.bookmarks, token),
        // `set` may target any slot, so keep the full digit list.
        _ => plain_candidates(
            match_filter(&["0","1","2","3","4","5","6","7","8","9"], token),
            CandidateKind::Word,
        ),
    }
}
```

```rust
/// Candidates for `bookmark go <slot>`: only the slots that hold a path, each
/// carrying that path as its hint, filtered by the digit typed so far and
/// ranked like any other word candidate.
fn bookmark_slot_candidates(
    bookmarks: &[Option<std::path::PathBuf>],
    token: &str,
) -> Vec<Candidate> {
    let populated: Vec<(String, String)> = bookmarks
        .iter()
        .enumerate()
        .filter_map(|(i, slot)| {
            let path = slot.as_ref()?;
            Some((i.to_string(), path.display().to_string()))
        })
        .collect();

    // Rank the slot digits through the shared matcher so ordering matches every
    // other source, then reattach each slot's path as its hint.
    let digits: Vec<String> = populated.iter().map(|(d, _)| d.clone()).collect();
    matcher::rank(token, digits)
        .into_iter()
        .map(|digit| {
            let hint = populated
                .iter()
                .find(|(d, _)| *d == digit)
                .map(|(_, p)| p.clone());
            Candidate { text: digit, kind: CandidateKind::Word, hint }
        })
        .collect()
}
```

In `src/popup.rs`, both completion call sites build a context from `App`:

```rust
// Tab arm:
let cwd = app.get_active_panel().path.clone();
let ctx = crate::complete::CompletionContext { cwd: &cwd, bookmarks: &app.bookmarks };
let Some(comp) = crate::complete::complete_with(&current, &ctx) else { return true; };
```

Apply the same in `refresh_menu`. Mind the borrow: clone `cwd` and read
`app.bookmarks` before the `&mut app.overlay` re-borrow, exactly as the buffer
clone already does.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib` then `cargo fmt --all -- --check` and
`cargo clippy --lib`. Expected: PASS. Manually confirm no test still asserts the
old blind-digit `bookmark go` behaviour.

- [ ] **Step 5: Commit**

```bash
git status --short
git add src/complete.rs src/complete/tests.rs src/popup.rs
git diff --cached --stat
git commit -m "feat: bookmark go completion lists populated slots with path hints

- [+] complete: bookmark go <Tab> offers only slots that hold a path, each
  showing the path as a dropdown hint, ranked through the shared matcher
- [!] bookmark go completion now needs the real slots from CompletionContext;
  the context-free wrapper no longer offers blind digits 0-9 for go (set keeps
  the full digit list)"
git push
```

---

### Task 3: Record the panel-entries decision in the spec

**Files:**
- Modify: `docs/superpowers/specs/2026-07-25-smart-autocomplete-design.md`

- [ ] **Step 1: Update the spec**

In the phase-3 bullet and the `sources.rs` component, add a short resolution
note: the *active panel entries* source was dropped because `complete()`
already scans the active panel's directory for `cd`, making panel entries a
duplicate of that scan with no other command position to serve; `bookmark`
slots shipped as the phase-3 context source. Keep it factual, 2–3 sentences.

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/specs/2026-07-25-smart-autocomplete-design.md
git commit -m "docs: record that phase 3 ships bookmark slots, drops panel entries

- panel-entries source duplicated cd's existing directory scan (KISS); cd is
  the only path argument in the command DSL, so it had no other home"
git push
```

---

## Verification

After Task 3:

```bash
cargo fmt --all -- --check   # clean
cargo clippy --lib           # no warnings
cargo test --lib --quiet     # all green
```

Manual check: `ffm`, set a bookmark (`:bookmark set 3`), then `:bookmark go `
and press Tab — the dropdown lists slot 3 with its path shown to the right;
empty slots are absent.

## Not in this phase / feature complete

With phase 3 done the approved spec is fully delivered: fuzzy matching +
dropdown (phase 1), history + ghost text (phase 2), bookmark-slot context
source (phase 3). The panel-entries source is intentionally not built.
