//! CLI flags for recordings: `--record`, `--no-record`, `--replay`.

use super::*;
use std::ffi::OsString;
use tempfile::TempDir;

fn os(s: &str) -> OsString {
    OsString::from(s)
}

#[test]
fn no_record_is_live_without_a_file() {
    let input = Input::from_args(&[os("--no-record")], (80, 24)).expect("args");
    assert!(matches!(input, Input::Live(None)));
    let options = input.terminal_options();
    assert!(matches!(options.viewport, ratatui::Viewport::Fullscreen));
}

#[test]
fn record_creates_a_session_file() {
    let dir = TempDir::new().expect("dir");
    let path = dir.path().join("session.py");
    let input = Input::from_args(&[os("--record"), path.as_os_str().to_os_string()], (40, 12))
        .expect("record");
    assert!(matches!(input, Input::Live(Some(_))));
    assert!(path.is_file());
}

#[test]
fn replay_loads_a_jsonl_recording() {
    let dir = TempDir::new().expect("dir");
    let path = dir.path().join("events.jsonl");
    let header = Header {
        version: 1,
        executable: dir.path().join("ffm"),
        directory: dir.path().to_path_buf(),
        size: (80, 24),
    };
    std::fs::write(
        &path,
        format!("{}\n", serde_json::to_string(&header).expect("json")),
    )
    .expect("write");
    let input = Input::from_args(&[os("--replay"), path.as_os_str().to_os_string()], (80, 24))
        .expect("replay");
    assert!(matches!(input, Input::Replay(_)));
    assert!(matches!(
        input.terminal_options().viewport,
        ratatui::Viewport::Fixed(_)
    ));
}

#[test]
fn bad_flags_and_bad_recordings_are_errors() {
    assert!(Input::from_args(&[os("--help")], (80, 24)).is_err());
    assert!(Input::from_args(&[os("--replay")], (80, 24)).is_err());
    let dir = TempDir::new().expect("dir");
    let empty = dir.path().join("empty.jsonl");
    std::fs::write(&empty, "").expect("empty");
    assert!(Replay::load(&empty, (80, 24)).is_err());
    let version = dir.path().join("v2.jsonl");
    std::fs::write(
        &version,
        "{\"version\":2,\"executable\":\"x\",\"directory\":\"/\",\"size\":[80,24]}\n",
    )
    .expect("v2");
    assert!(Replay::load(&version, (80, 24)).is_err());
}

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

/// Recording is opt-in: a plain launch writes no file. A recording holds every
/// character typed, including anything pasted into a prompt.
#[test]
fn empty_args_do_not_start_a_recording() {
    let input = Input::from_args(&[], (80, 24)).expect("default");
    assert!(matches!(input, Input::Live(None)));
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

#[test]
fn live_read_without_a_tty_does_not_panic() {
    let mut input = Input::from_args(&[os("--no-record")], (80, 24)).expect("live");
    let _ = input.read();
}

fn write_replay(dir: &TempDir, delay_ms: u64) -> PathBuf {
    let path = dir.path().join("events.jsonl");
    let header = Header {
        version: 1,
        executable: dir.path().join("ffm"),
        directory: dir.path().to_path_buf(),
        size: (80, 24),
    };
    let event = RecordedEvent {
        delay_ms,
        event: Event::Resize(80, 24),
    };
    std::fs::write(
        &path,
        format!(
            "{}\n{}\n",
            serde_json::to_string(&header).expect("header"),
            serde_json::to_string(&event).expect("event")
        ),
    )
    .expect("write");
    path
}

#[test]
fn replay_with_a_delay_returns_none_until_due() {
    let dir = TempDir::new().expect("dir");
    let mut replay = Replay::load(&write_replay(&dir, 50), (80, 24)).expect("load");
    assert_eq!(replay.read().expect("wait"), None);
    let got = loop {
        if let Ok(Some(event)) = replay.read() {
            break event;
        }
    };
    assert!(matches!(got, Event::Resize(80, 24)));
}

#[test]
fn replay_input_read_yields_the_recorded_event() {
    let dir = TempDir::new().expect("dir");
    let path = write_replay(&dir, 0);
    let mut input =
        Input::from_args(&[os("--replay"), path.into_os_string()], (80, 24)).expect("replay");
    assert!(matches!(
        input.read().expect("event"),
        Some(Event::Resize(80, 24))
    ));
}
