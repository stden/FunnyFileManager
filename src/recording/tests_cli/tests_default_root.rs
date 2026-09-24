//! Purpose: `--record` without a path — the private auto-named root, and a recording that cannot start never stops the session.

use super::*;

/// The auto-named path lives under the temporary directory, in a directory of
/// this user's own: the shared name was owned by whoever ran ffm first, which
/// left everyone else unable to start at all.
#[test]
fn the_default_record_root_is_private_to_this_user() {
    let root = default_root();
    assert!(root.starts_with(std::env::temp_dir()));
    let name = root
        .file_name()
        .expect("a directory name")
        .to_string_lossy()
        .into_owned();
    assert!(name.contains("ffm-recordings"), "unexpected name: {name}");
    #[cfg(unix)]
    assert_ne!(
        name, "ffm-recordings",
        "the name must not be shared between users"
    );
}

/// The directory holds keystrokes, so it is created 0700 whatever the umask.
#[cfg(unix)]
#[test]
fn the_default_record_root_is_created_private() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().expect("dir");
    let root = dir.path().join("nested").join("recordings");
    create_private_dir(&root).expect("create");
    let mode = std::fs::metadata(&root)
        .expect("metadata")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o700, "mode was {:o}", mode & 0o777);
}

/// `--record` on its own records to the auto-named path. The file it makes is
/// real, so the test takes it away again: every other test here works inside a
/// `TempDir`, and this one would otherwise leave a session behind on each run.
#[test]
fn bare_record_uses_the_default_path() {
    let before = session_files();
    let input = Input::from_args(&[os("--record")], (80, 24)).expect("record");
    assert!(matches!(input, Input::Live(Some(_))));

    let made: Vec<_> = session_files()
        .into_iter()
        .filter(|path| !before.contains(path))
        .collect();
    assert_eq!(made.len(), 1, "one session file: {made:?}");
    for path in made {
        std::fs::remove_file(path).expect("clean up");
    }
}

/// The session files under the auto-named root, as full paths.
fn session_files() -> Vec<PathBuf> {
    std::fs::read_dir(default_root())
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .collect()
}

/// A session file that cannot be created is a disabled recording, never a
/// refusal to start: that is exactly what locked every other user out.
#[test]
fn an_unusable_recording_path_still_starts_the_session() {
    let dir = TempDir::new().expect("dir");
    let path = dir.path().join("no-such-directory").join("session.py");
    let input = Input::from_args(&[os("--record"), path.as_os_str().to_os_string()], (80, 24))
        .expect("a bad recording path must not fail the start");
    assert!(matches!(input, Input::Live(None)));
}
