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

/// `--record` on its own records to an auto-named session in the recordings
/// root, creating the root. The root is a `TempDir` here: the real one, in the
/// shared temporary directory, is left to the user's own sessions.
#[test]
fn bare_record_names_a_session_in_the_recordings_root() {
    let dir = TempDir::new().expect("dir");
    let root = dir.path().join("recordings");
    let (input, disabled) =
        Input::from_args_in(&[os("--record")], (80, 24), &root).expect("record");
    assert!(matches!(input, Input::Live(Some(_))));
    assert!(disabled.is_none());

    let made: Vec<_> = std::fs::read_dir(&root)
        .expect("the root was created")
        .map(|entry| entry.expect("entry").file_name())
        .collect();
    assert_eq!(made.len(), 1, "one session file: {made:?}");
}

/// A session file that cannot be created is a disabled recording, never a
/// refusal to start: that is exactly what locked every other user out.
#[test]
fn an_unusable_recording_path_still_starts_the_session() {
    let dir = TempDir::new().expect("dir");
    let path = dir.path().join("no-such-directory").join("session.py");
    let (input, disabled) =
        Input::from_args(&[os("--record"), path.as_os_str().to_os_string()], (80, 24))
            .expect("a bad recording path must not fail the start");
    assert!(matches!(input, Input::Live(None)));
    let reason = disabled.expect("the reason is kept for the status line");
    assert!(
        reason.to_string().starts_with("recording disabled: "),
        "{reason}"
    );
}

/// A recordings root that is not this user's own directory — here a link
/// planted to redirect the keystrokes — disables the recording, says why, and
/// writes nothing through it.
#[cfg(unix)]
#[test]
fn a_recordings_root_that_is_not_our_own_disables_the_recording() {
    let dir = TempDir::new().expect("dir");
    let elsewhere = dir.path().join("elsewhere");
    std::fs::create_dir(&elsewhere).expect("elsewhere");
    let root = dir.path().join("recordings");
    std::os::unix::fs::symlink(&elsewhere, &root).expect("plant the link");

    let (input, disabled) =
        Input::from_args_in(&[os("--record")], (80, 24), &root).expect("the session starts");

    assert!(matches!(input, Input::Live(None)));
    let reason = disabled.expect("the reason is kept for the status line");
    assert!(
        reason
            .to_string()
            .starts_with("recording disabled: not a directory of your own: "),
        "{reason}"
    );
    let written = std::fs::read_dir(&elsewhere).expect("elsewhere").count();
    assert_eq!(written, 0, "nothing is written through the link");
}
