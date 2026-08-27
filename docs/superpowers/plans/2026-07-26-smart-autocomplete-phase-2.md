<!-- Purpose: Phase 2 implementation plan for smart autocomplete (history persistence + ghost text). -->

# Smart Autocomplete Phase 2 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Executed `:` commands are remembered across runs and the most recent
match shows as grey ghost text ahead of the cursor; `Right`/`End` accepts it.

**Architecture:** A new `src/history.rs` owns the store: an in-memory ring of
everything executed this session plus a disk file holding only lines whose
durability the type system has proven safe. `App` gains a `history: History`
field; `popup.rs` records on Enter and recomputes the ghost on every buffer
edit; `ui.rs` renders the ghost as a dimmed span after the buffer.

**Tech Stack:** Rust 2021 (rust-version 1.95), `ratatui` 0.30. No new
dependencies — XDG paths via `std::env`, no `dirs` crate.

## Global Constraints

- No file in `src/` may exceed 1000 lines; split at ~500 into modules. Unit
  tests live in `<module>/tests*.rs` child modules.
- `cargo fmt --all -- --check` and `cargo clippy --lib` clean (pedantic +
  nursery: short first doc paragraph, backticked identifiers in docs).
- TDD for every task: failing test first, watch it fail, minimal code, green.
- **Secrets rule (spec §Secrets, keepass.rs §doc):** anything typed at `:`
  that parses to `Command::Shell` or `Command::Invalid` must NEVER reach
  disk. The gate is `Command::parse`, not regex guessing. This plan encodes
  the gate as a type: the disk writer's signature only accepts
  `DurableLine`, and `DurableLine`'s only constructor consults
  `Command::parse`. (Project style directive: enums, newtypes, DSL.)
- History I/O must never break the file manager: read/write failures are
  swallowed; a missing or corrupt file is an empty history.
- Ghost text is **literal-prefix**, never fuzzy: `Right` accepts it without
  confirmation, so it must be predictable (spec §history.rs).
- Ghost is suppressed while the dropdown menu is open (spec: never two
  competing accept-targets for one `Enter`).
- This repo is edited by a concurrent agent. Stage only the files each task
  names; never `git add -A`. Run `git status --short` before staging and
  `git diff --cached --stat` after. Push after each task's commit.

## File Structure

| File | Responsibility |
|---|---|
| `src/history.rs` (create) | `History`, `DurableLine` newtype, XDG file I/O, `suggest` |
| `src/history/tests.rs` (create) | Store, gate, cap/dedup, corrupt-file, suggest tests |
| `src/models.rs` (modify) | `App.history: History`; `CommandLine.ghost: Option<String>` |
| `src/popup.rs` (modify) | Record on Enter; ghost recompute on edits; `Right`/`End` accept |
| `src/ui.rs` (modify) | Render ghost as a dimmed span after the buffer |
| `src/lib.rs` (modify) | (only if `App::new` call sites need nothing — expected no change) |

---

### Task 1: `History` store with a type-gated disk file

**Files:**
- Create: `src/history.rs`
- Create: `src/history/tests.rs`
- Modify: `src/lib.rs` (add `pub mod history;` to the module list)

**Interfaces:**
- Consumes: `crate::command::Command` (its `parse` is the durability gate).
- Produces (later tasks rely on these exact signatures):
  - `pub struct History { .. }` with `impl Default`
  - `History::load() -> History` — reads the XDG file, never fails
  - `History::record(&mut self, line: &str)` — remembers in-session always;
    appends to disk only when durable
  - `History::suggest(&self, prefix: &str) -> Option<&str>` — most recent
    entry (session ring first, then persisted, newest first) having `prefix`
    as a **literal, non-empty-tail** prefix; returns the FULL entry
  - `pub struct DurableLine(..)` — newtype; only constructor
    `DurableLine::classify(line: &str) -> Option<DurableLine>` returns
    `Some` iff `Command::parse` yields a known verb (not `Shell`, not
    `Invalid`)
  - `const HISTORY_CAP: usize = 500`

- [ ] **Step 1: Write the failing tests**

Create `src/history/tests.rs`:

```rust
//! Unit tests: command history store, durability gate, suggestions.
#![allow(unused_imports)]

use super::*;
use std::fs;
use tempfile::TempDir;

/// Redirect `XDG_DATA_HOME` to a tempdir for the duration of a test.
/// History file I/O reads the variable at call time, so tests must
/// serialize access — reuse the global env lock from `test_support`.
fn with_xdg<T>(f: impl FnOnce(&std::path::Path) -> T) -> T {
    let _guard = crate::test_support::editor_env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let tmp = TempDir::new().expect("tempdir");
    let old = std::env::var_os("XDG_DATA_HOME");
    std::env::set_var("XDG_DATA_HOME", tmp.path());
    let out = f(tmp.path());
    match old {
        Some(v) => std::env::set_var("XDG_DATA_HOME", v),
        None => std::env::remove_var("XDG_DATA_HOME"),
    }
    out
}

#[test]
fn durable_line_accepts_known_verbs_only() {
    assert!(DurableLine::classify("cd /tmp").is_some());
    assert!(DurableLine::classify("sort size").is_some());
    assert!(DurableLine::classify("bookmark go 3").is_some());
    // Shell fall-through and known-verb-bad-args must never be durable.
    assert!(DurableLine::classify("aws s3 cp x --token SECRET").is_none());
    assert!(DurableLine::classify("bookmark set 99").is_none());
    assert!(DurableLine::classify("").is_none());
}

#[test]
fn record_persists_durable_lines_across_loads() {
    with_xdg(|_| {
        let mut h = History::load();
        h.record("cd /tmp");
        h.record("sort size");

        let reloaded = History::load();
        assert_eq!(reloaded.suggest("cd"), Some("cd /tmp"));
        assert_eq!(reloaded.suggest("so"), Some("sort size"));
    });
}

#[test]
fn shell_lines_stay_in_session_and_off_disk() {
    with_xdg(|xdg| {
        let mut h = History::load();
        h.record("aws s3 cp x s3://b --token SECRET");

        // Usable this session…
        assert_eq!(
            h.suggest("aws"),
            Some("aws s3 cp x s3://b --token SECRET")
        );
        // …but nothing of it on disk, and not seen by a fresh load.
        let file = xdg.join("ffm/history");
        let on_disk = fs::read_to_string(&file).unwrap_or_default();
        assert!(!on_disk.contains("SECRET"), "disk: {on_disk}");
        assert_eq!(History::load().suggest("aws"), None);
    });
}

#[test]
fn suggest_is_literal_prefix_and_newest_wins() {
    with_xdg(|_| {
        let mut h = History::load();
        h.record("cd /old");
        h.record("cd /new");

        assert_eq!(h.suggest("cd /"), Some("cd /new"));
        // Literal prefix only — no fuzzy: "c/" is not a prefix of anything.
        assert_eq!(h.suggest("c/"), None);
        // A prefix equal to a full entry has no tail to offer.
        assert_eq!(h.suggest("cd /new"), None);
    });
}

#[test]
fn duplicate_records_keep_only_the_newest() {
    with_xdg(|xdg| {
        let mut h = History::load();
        h.record("cd /tmp");
        h.record("pwd");
        h.record("cd /tmp");

        let on_disk = fs::read_to_string(xdg.join("ffm/history")).expect("file");
        assert_eq!(
            on_disk.matches("cd /tmp").count(),
            1,
            "dedup keeps one copy: {on_disk}"
        );
        // …and it is the newest, so it wins over "pwd" for a "" prefix? No:
        // empty prefix has no tail rule — assert via the concrete prefix.
        assert_eq!(h.suggest("cd"), Some("cd /tmp"));
    });
}

#[test]
fn the_file_is_capped() {
    with_xdg(|xdg| {
        let mut h = History::load();
        for i in 0..(HISTORY_CAP + 50) {
            h.record(&format!("cd /dir{i}"));
        }
        let on_disk = fs::read_to_string(xdg.join("ffm/history")).expect("file");
        assert_eq!(on_disk.lines().count(), HISTORY_CAP);
        // Oldest lines fell off, newest survived.
        assert!(!on_disk.contains("/dir0\n"), "oldest should be gone");
        assert!(
            on_disk.contains(&format!("cd /dir{}\n", HISTORY_CAP + 49)),
            "newest line survives"
        );
    });
}

#[test]
fn corrupt_or_missing_file_is_an_empty_history() {
    with_xdg(|xdg| {
        // Missing file.
        assert_eq!(History::load().suggest("cd"), None);
        // Corrupt file (invalid UTF-8) must not panic and must not leak junk.
        fs::create_dir_all(xdg.join("ffm")).expect("mkdir");
        fs::write(xdg.join("ffm/history"), [0xFF, 0xFE, 0x00]).expect("write");
        let h = History::load();
        assert_eq!(h.suggest("cd"), None);
    });
}

#[cfg(unix)]
#[test]
fn the_history_file_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    with_xdg(|xdg| {
        let mut h = History::load();
        h.record("cd /tmp");
        let mode = fs::metadata(xdg.join("ffm/history"))
            .expect("file")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "history holds the user's paths");
    });
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib history`
Expected: FAIL to compile — `file not found for module 'history'` /
`cannot find History`.

- [ ] **Step 3: Write the minimal implementation**

Create `src/history.rs`:

```rust
//! Purpose: Persistent `:` command history with a type-gated durability rule.
//!
//! Everything executed is remembered for the session; only lines the
//! [`DurableLine`] gate accepts are written to disk. The gate consults
//! [`Command::parse`](crate::command::Command::parse): `Shell` and `Invalid`
//! lines — the ones that can carry passwords and tokens — cannot even be
//! *represented* as durable, so no code path can write them. See the secrets
//! rule in `keepass.rs` and the design spec.

use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

use crate::command::Command;

/// Most lines the on-disk history keeps; oldest fall off first.
pub const HISTORY_CAP: usize = 500;

/// A command line proven safe to persist.
///
/// The only way to obtain one is [`DurableLine::classify`], which returns
/// `None` for `Command::Shell` and `Command::Invalid` — so the file writer,
/// whose signature demands `&[DurableLine]`, cannot receive a secret even by
/// a future refactoring mistake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableLine(String);

impl DurableLine {
    /// Classify `line`; `Some` iff it parses to a known verb.
    #[must_use]
    pub fn classify(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        match Command::parse(trimmed) {
            Command::Shell(_) | Command::Invalid(_) => None,
            _ => Some(Self(trimmed.to_owned())),
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Session-plus-disk command history.
///
/// `session` holds everything executed this run, newest last. `durable`
/// mirrors the on-disk file (newest last). Suggestions search the session
/// ring first — most recent experience wins — then the persisted lines.
#[derive(Debug, Default)]
pub struct History {
    session: VecDeque<String>,
    durable: VecDeque<DurableLine>,
}

impl History {
    /// Read the persisted history. Missing, unreadable, or corrupt files are
    /// an empty history — completion must never break the file manager.
    #[must_use]
    pub fn load() -> Self {
        let durable = history_file_path()
            .and_then(|p| fs::read_to_string(p).ok())
            .map(|text| {
                text.lines()
                    // classify() re-checks on load: a hand-edited file cannot
                    // smuggle a shell line back into the durable set.
                    .filter_map(DurableLine::classify)
                    .collect()
            })
            .unwrap_or_default();
        Self {
            session: VecDeque::new(),
            durable,
        }
    }

    /// Remember an executed line. Always kept for the session; appended to
    /// disk only when [`DurableLine::classify`] accepts it.
    pub fn record(&mut self, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return;
        }
        push_deduped(&mut self.session, trimmed.to_owned(), |a, b| a == b);

        if let Some(durable) = DurableLine::classify(trimmed) {
            push_deduped(&mut self.durable, durable, PartialEq::eq);
            save(self.durable.make_contiguous());
        }
    }

    /// The most recent entry having `prefix` as a literal prefix and more
    /// text after it. Literal, never fuzzy: `Right` accepts the result
    /// without confirmation, so it must be exactly what it looks like.
    #[must_use]
    pub fn suggest(&self, prefix: &str) -> Option<&str> {
        if prefix.is_empty() {
            return None;
        }
        self.session
            .iter()
            .rev()
            .map(String::as_str)
            .chain(self.durable.iter().rev().map(DurableLine::as_str))
            .find(|entry| entry.starts_with(prefix) && entry.len() > prefix.len())
    }
}

/// Push `value`, dropping any older equal entry (dedup keep-newest) and the
/// oldest lines beyond [`HISTORY_CAP`].
fn push_deduped<T>(ring: &mut VecDeque<T>, value: T, eq: impl Fn(&T, &T) -> bool) {
    ring.retain(|existing| !eq(existing, &value));
    ring.push_back(value);
    while ring.len() > HISTORY_CAP {
        ring.pop_front();
    }
}

/// `$XDG_DATA_HOME/ffm/history`, defaulting to `~/.local/share/ffm/history`.
fn history_file_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(base.join("ffm/history"))
}

/// Rewrite the history file from `lines`, owner-only where the OS supports
/// it. All failures are swallowed: losing history is acceptable, breaking
/// the file manager is not.
fn save(lines: &[DurableLine]) {
    let Some(path) = history_file_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        if fs::create_dir_all(dir).is_err() {
            return;
        }
    }
    let mut text = String::new();
    for line in lines {
        text.push_str(line.as_str());
        text.push('\n');
    }
    let _ = write_owner_only(&path, &text);
}

#[cfg(unix)]
fn write_owner_only(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(text.as_bytes())
}

#[cfg(not(unix))]
fn write_owner_only(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    fs::write(path, text)
}

#[cfg(test)]
mod tests;
```

Add `pub mod history;` to `src/lib.rs`'s module list (alphabetical spot,
after `pub mod handlers;`).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib history`
Expected: PASS (8 tests). Then `cargo clippy --lib` — clean. If
`set_var`/`remove_var` are flagged unsafe in this edition setup, follow the
existing pattern in `src/test_support.rs` for env mutation.

- [ ] **Step 5: Commit**

```bash
git status --short
git add src/history.rs src/history/tests.rs src/lib.rs
git diff --cached --stat
git commit -m "feat: type-gated persistent command history

- [+] history::History — session ring + XDG-persisted lines (cap 500,
  dedup keep-newest, 0600, corrupt file = empty history)
- [+] history::DurableLine — newtype whose only constructor consults
  Command::parse, so Shell/Invalid lines (the ones that can carry
  secrets) cannot be represented as writable-to-disk at all"
git push
```

---

### Task 2: `App` carries history; Enter records through it

**Files:**
- Modify: `src/models.rs` (field + `App::new`)
- Modify: `src/popup.rs` (the unguarded `Enter` arm records)
- Modify (only if the compiler demands): test files constructing `App { .. }`
  literals gain `history: History::default(),`

**Interfaces:**
- Consumes: `History` (Task 1).
- Produces: `App.history: crate::history::History` — public field, loaded in
  `App::new` via `History::load()`.

- [ ] **Step 1: Write the failing test**

Append to `src/handlers/tests_popup_command_line.rs`:

```rust
#[test]
fn executed_commands_enter_the_session_history() {
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::CommandLine("pwd".into()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert_eq!(app.history.suggest("pw"), Some("pwd"));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib executed_commands_enter_the_session_history`
Expected: FAIL to compile — no field `history` on `App`.

- [ ] **Step 3: Write the minimal implementation**

`src/models.rs`: add to the `App` struct (documented like its neighbours):

```rust
    /// Executed `:` commands, session-wide, persisted per the durability gate.
    pub history: crate::history::History,
```

In `App::new`, add `history: crate::history::History::load(),` to the
literal. Fix any `App { .. }` literals the compiler reports in test files by
adding `history: crate::history::History::default(),` (alias the import to
taste).

`src/popup.rs`, the **unguarded** `Enter` arm — record before executing:

```rust
                KeyCode::Enter => {
                    let command = state.buffer.trim().to_string();
                    app.close_overlay();
                    if !command.is_empty() {
                        app.history.record(&command);
                        if let Err(err) = crate::handlers::execute_command(app, &command) {
                            app.set_status(crate::status::Status::Error(err.to_string()));
                        }
                    }
                }
```

(Recording failures is deliberate: fish records failed commands too, and an
`Invalid` line stays session-only via the gate.)

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS, prior count + 1. `cargo fmt --all` + `cargo clippy --lib`
clean.

- [ ] **Step 5: Commit**

```bash
git status --short
git add src/models.rs src/popup.rs src/handlers/tests_popup_command_line.rs
# plus any test files the compiler forced you to touch — list them explicitly
git diff --cached --stat
git commit -m "feat: App carries command history; Enter records through it

- [!] App.history — loaded from disk at startup, session ring always live
- [!] command-line Enter records the line before executing (Invalid/Shell
  stay session-only via the DurableLine gate)"
git push
```

---

### Task 3: Ghost text state and keys

**Files:**
- Modify: `src/models.rs` (`CommandLine.ghost: Option<String>`)
- Modify: `src/popup.rs` (ghost recompute + `Right`/`End` accept)
- Modify: `src/handlers/tests_popup_command_line.rs` (behaviour tests)

**Interfaces:**
- Consumes: `History::suggest` (Task 1), `App.history` (Task 2).
- Produces:
  - `CommandLine { buffer, menu, ghost: Option<String> }` — `ghost` is the
    suggestion **tail** (text beyond the buffer), `None` when there is none
    or while the menu is open
  - `fn refresh_ghost(app: &mut App)` in popup.rs (private helper)

- [ ] **Step 1: Write the failing tests**

Append to `src/handlers/tests_popup_command_line.rs`:

```rust
/// Seed history, open a fresh command line, and return the App.
fn app_with_history(lines: &[&str]) -> App {
    let mut app = App::new().expect("test app");
    for line in lines {
        app.history.record(line);
    }
    app.set_overlay(Overlay::CommandLine(CommandLine::default()));
    app
}

#[test]
fn typing_shows_the_most_recent_matching_tail_as_ghost() {
    let mut app = app_with_history(&["cd /somewhere"]);

    process_popup_input(&mut app, key_event(KeyCode::Char('c')));
    process_popup_input(&mut app, key_event(KeyCode::Char('d')));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line open");
    };
    assert_eq!(state.buffer, "cd");
    assert_eq!(state.ghost.as_deref(), Some(" /somewhere"));
}

#[test]
fn right_accepts_the_ghost_into_the_buffer() {
    let mut app = app_with_history(&["cd /somewhere"]);
    process_popup_input(&mut app, key_event(KeyCode::Char('c')));

    process_popup_input(&mut app, key_event(KeyCode::Right));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line open");
    };
    assert_eq!(state.buffer, "cd /somewhere");
    assert_eq!(state.ghost, None, "tail fully consumed");
}

#[test]
fn backspace_recomputes_the_ghost() {
    let mut app = app_with_history(&["pwd", "pull"]);
    // "p" → newest match "pull" wins; then extend to "pw" → only "pwd".
    process_popup_input(&mut app, key_event(KeyCode::Char('p')));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert_eq!(state.ghost.as_deref(), Some("ull"));

    process_popup_input(&mut app, key_event(KeyCode::Char('w')));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert_eq!(state.ghost.as_deref(), Some("d"));

    process_popup_input(&mut app, key_event(KeyCode::Backspace));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert_eq!(state.buffer, "p");
    assert_eq!(state.ghost.as_deref(), Some("ull"));
}

#[test]
fn ghost_is_suppressed_while_the_menu_is_open() {
    let mut app = app_with_history(&["mark all"]);
    process_popup_input(&mut app, key_event(KeyCode::Char('m')));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert!(state.ghost.is_some(), "ghost shows while menu closed");

    // Tab opens the menu ("m" is ambiguous among verbs) — ghost must clear.
    process_popup_input(&mut app, key_event(KeyCode::Tab));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert!(state.menu.is_some());
    assert_eq!(state.ghost, None, "no two accept-targets at once");

    // Esc closes the menu; the ghost returns for the unchanged buffer.
    process_popup_input(&mut app, key_event(KeyCode::Esc));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert!(state.menu.is_none());
    assert!(state.ghost.is_some(), "ghost returns once the menu closes");
}

#[test]
fn right_without_a_ghost_is_swallowed_and_harmless() {
    let mut app = app_with_history(&[]);
    process_popup_input(&mut app, key_event(KeyCode::Char('z')));

    assert!(process_popup_input(&mut app, key_event(KeyCode::Right)));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert_eq!(state.buffer, "z");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib tests_popup_command_line`
Expected: FAIL to compile — no field `ghost` on `CommandLine`.

- [ ] **Step 3: Write the minimal implementation**

`src/models.rs` — extend `CommandLine` (both `From` impls and any struct
literals set `ghost: None`):

```rust
pub struct CommandLine {
    pub buffer: String,
    /// `None` while the dropdown is closed.
    pub menu: Option<Menu>,
    /// History suggestion tail beyond `buffer` (grey text after the cursor).
    /// `None` when nothing matches or while the menu is open — the menu and
    /// the ghost must never compete for the same accept gesture.
    pub ghost: Option<String>,
}
```

`src/popup.rs` — helper beside `refresh_menu`:

```rust
/// Recompute the ghost suggestion for the current buffer.
///
/// Cleared while the menu is open: the ghost's accept key would fight the
/// menu's selection for the user's intent. Literal-prefix only, via
/// [`crate::history::History::suggest`].
fn refresh_ghost(app: &mut App) {
    let Overlay::CommandLine(state) = &app.overlay else {
        return;
    };
    if state.menu.is_some() {
        if let Overlay::CommandLine(state) = &mut app.overlay {
            state.ghost = None;
        }
        return;
    }
    let buffer = state.buffer.clone();
    let ghost = app
        .history
        .suggest(&buffer)
        .map(|entry| entry[buffer.len()..].to_owned());
    if let Overlay::CommandLine(state) = &mut app.overlay {
        state.ghost = ghost;
    }
}
```

Key arms in the `Overlay::CommandLine` match:

- `Char` arm: after the existing `refresh_menu(app)` conditional, call
  `refresh_ghost(app);` unconditionally (it self-clears when a menu is open).
- `Backspace` arm: same — `refresh_ghost(app);` after the menu refresh.
- `Tab` arms (both): after the menu state settles, call `refresh_ghost(app);`
  (opening a menu clears the ghost; a lone-candidate accept recomputes it).
- `Esc` arm: when only the menu was closed, call `refresh_ghost(app);` so the
  ghost returns.
- New arms, placed with the other guarded arms (before the unguarded `Enter`):

```rust
                KeyCode::Right | KeyCode::End if state.ghost.is_some() => {
                    let tail = state.ghost.take().expect("ghost is some");
                    state.buffer.push_str(&tail);
                    refresh_ghost(app);
                }
```

Note `state.ghost.take()` ends the `state` borrow before `refresh_ghost(app)`
— same discipline as every other arm.

Buffer-length slicing safety: `suggest` guarantees `entry.starts_with(buffer)`,
so `entry[buffer.len()..]` cuts at a char boundary by construction.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS, prior count + 5. fmt + clippy clean.

- [ ] **Step 5: Commit**

```bash
git status --short
git add src/models.rs src/popup.rs src/handlers/tests_popup_command_line.rs
git diff --cached --stat
git commit -m "feat: history ghost text on the command line

- [+] CommandLine.ghost — literal-prefix history tail, recomputed on every
  edit, suppressed while the menu is open (one accept-target at a time)
- [+] Right/End accept the ghost into the buffer"
git push
```

---

### Task 4: Render the ghost

**Files:**
- Modify: `src/ui.rs` (`render_command_line` appends a dimmed span)
- Modify: `src/ui/tests.rs` (render assertions)

**Interfaces:**
- Consumes: `CommandLine.ghost` (Task 3).
- Produces: no new API.

- [ ] **Step 1: Write the failing tests**

Append to `src/ui/tests.rs` (pin panel paths like the neighbouring tests so
assertions are environment-independent):

```rust
#[test]
fn test_ghost_text_renders_after_the_buffer() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");
    app.set_overlay(Overlay::CommandLine(CommandLine {
        buffer: "cd".to_string(),
        menu: None,
        ghost: Some(" /somewhere".to_string()),
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(screen.contains("cd /somewhere"), "{screen}");
}

#[test]
fn test_no_ghost_renders_nothing_extra() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");
    app.set_overlay(Overlay::CommandLine("cd".into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(!screen.contains("cd /somewhere"), "{screen}");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib test_ghost_text_renders_after_the_buffer`
Expected: FAIL — assertion; nothing draws the ghost yet. (The second test
passes trivially now; it exists as the negative guard once rendering lands.)

- [ ] **Step 3: Write the minimal implementation**

In `render_command_line` (`src/ui.rs`): read the ghost alongside the buffer,

```rust
    let (cmd_text, ghost) = if let Overlay::CommandLine(ref state) = app.overlay {
        (state.buffer.as_str(), state.ghost.as_deref().unwrap_or(""))
    } else {
        ("", "")
    };
```

include the ghost's width in the `reserved` computation (it takes real
columns), and add one span after the cursor span:

```rust
        Span::styled(
            ghost.to_owned(),
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        ),
```

Keep the padding arithmetic in step: `used` must count
`ghost.chars().count()` too, or the padding span will push the row past the
terminal edge.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS, prior count + 2. Full gate:
`cargo fmt --all -- --check && cargo clippy --lib && cargo test --lib --quiet`.

- [ ] **Step 5: Commit**

```bash
git status --short
git add src/ui.rs src/ui/tests.rs
git diff --cached --stat
git commit -m "feat: render the history ghost after the typed command

- [+] render_command_line — DarkGray suggestion tail after the cursor,
  width-accounted so deep prompts still truncate correctly"
git push
```

---

## Verification

After Task 4:

```bash
cargo fmt --all -- --check   # clean
cargo clippy --lib           # no warnings
cargo test --lib --quiet     # all green
```

Manual check in a real terminal: run `ffm`, execute `:cd /tmp`, quit,
relaunch, press `:` and type `c` — grey ` d /tmp`-style tail appears;
`Right` accepts it. Then type a shell line, quit, and
`cat ~/.local/share/ffm/history` — the shell line must be absent.

## Not in this phase

Panel-entry and bookmark candidate sources (phase 3), fuzzy ghost, command
deletion from history, `Ctrl+R`-style history search.
