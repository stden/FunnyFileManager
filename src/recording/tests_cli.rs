//! CLI flags for recordings: `--record`, `--no-record`, `--replay`.

use super::*;
use std::ffi::OsString;
use tempfile::TempDir;

mod tests_default_root;

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

/// Recording is opt-in: a plain launch writes no file. A recording holds every
/// character typed, including anything pasted into a prompt.
#[test]
fn empty_args_do_not_start_a_recording() {
    let input = Input::from_args(&[], (80, 24)).expect("default");
    assert!(matches!(input, Input::Live(None)));
}

#[test]
fn live_read_without_a_tty_does_not_panic() {
    let mut input = Input::from_args(&[os("--no-record")], (80, 24)).expect("live");
    drop(input.read());
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
