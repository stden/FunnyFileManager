//! Purpose: history store shape — prefix suggestions, dedup, the on-disk cap, corrupt or missing files, and the never-persisting default.

use super::*;

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

/// Two ffm windows share one history file. Each save used to rewrite the file
/// from that window's own memory, so the other window's lines vanished.
#[test]
fn two_running_instances_keep_each_others_lines() {
    with_xdg(|xdg| {
        let mut first = History::load();
        let mut second = History::load();
        first.record("sort size");
        second.record("sort name");
        first.record("pwd");

        let on_disk = fs::read_to_string(xdg.join("ffm/history")).expect("file");
        assert_eq!(on_disk, "sort size\nsort name\npwd\n");
    });
}

/// Merging must not bring back what the other instance's save let fall off
/// or deduplicated: the file on disk is the starting point, not this
/// instance's memory of it.
#[test]
fn merging_keeps_the_newest_copy_of_a_repeated_line() {
    with_xdg(|xdg| {
        let mut first = History::load();
        let mut second = History::load();
        first.record("cd /a");
        second.record("pwd");
        second.record("cd /a");
        first.record("sort size");

        let on_disk = fs::read_to_string(xdg.join("ffm/history")).expect("file");
        assert_eq!(on_disk, "pwd\ncd /a\nsort size\n");
    });
}
