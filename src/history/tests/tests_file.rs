//! Purpose: history file safety — owner-only mode, XDG path resolution, whitespace no-ops, and swallowed save failures.

use super::*;

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
    let expected = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|home| home.join(".local/share/ffm/history"));
    assert_eq!(history_file_path(), expected);
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

/// The data directory the first save makes is readable by its owner only,
/// as the XDG spec asks: it was 0755, listing the histories to everyone.
#[cfg(unix)]
#[test]
fn the_first_save_makes_an_owner_only_data_directory() {
    use std::os::unix::fs::PermissionsExt;
    with_xdg(|xdg| {
        let mut history = History::load();
        history.record("cd /tmp");
        let mode = fs::metadata(xdg.join("ffm"))
            .expect("the data directory")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700);
    });
}
