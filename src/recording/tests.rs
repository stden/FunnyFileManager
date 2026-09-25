//! Recordings preserve input payloads without generating Python expressions.
use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::Path;

mod clipboard;
use tempfile::TempDir;

fn replay_file(script: &Path, destination: &Path) {
    let source = std::fs::read_to_string(script).expect("script");
    let rows: Vec<_> = source
        .lines()
        .filter_map(|line| line.strip_prefix("# FFM "))
        .collect();
    std::fs::write(destination, rows.join("\n")).expect("replay data");
}

#[test]
fn recorded_keys_replay_a_real_directory_creation_through_the_event_loop() {
    let root = TempDir::new().expect("directory");
    let script = root.path().join("session.py");
    let data = root.path().join("events.jsonl");
    let mut recorder = Recorder::create(&script, &header(root.path())).expect("recorder");
    record_directory_workflow(&mut recorder);
    replay_file(&script, &data);
    let mut replay = Replay::load(&data, (80, 24)).expect("replay");
    let mut app = crate::models::App::new().expect("app");
    app.left_panel = crate::models::Panel::new(root.path().to_path_buf()).expect("panel");
    app.right_panel = crate::models::Panel::new(root.path().to_path_buf()).expect("panel");
    let mut terminal =
        ratatui::Terminal::new(crate::WritableTestBackend::new(80, 24)).expect("terminal");
    crate::application::run_app_with_wsl_backend(
        &mut terminal,
        app,
        || replay.read(),
        std::sync::Arc::new(crate::wsl::SystemWslBackend),
    )
    .expect("replay application");
    assert!(root.path().join("x").is_dir());
    assert!(replay.events.is_empty());
}

fn header(root: &Path) -> Header {
    Header {
        version: 1,
        executable: root.join("ffm.exe"),
        directory: root.to_path_buf(),
        size: (80, 24),
    }
}

#[test]
fn recording_is_a_valid_python_script_with_lossless_event_data() {
    let root = TempDir::new().expect("temporary directory");
    let path = root.path().join("session.py");
    let mut recorder = Recorder::create(&path, &header(root.path())).expect("recorder");
    let events = [
        Event::Paste("Ж🙂\n'\"\\".to_owned()),
        Event::Resize(120, 40),
    ];
    for event in &events {
        recorder.record(event).expect("record");
    }
    let source = std::fs::read_to_string(&path).expect("script");
    let rows: Vec<_> = source
        .lines()
        .filter_map(|line| line.strip_prefix("# FFM "))
        .collect();
    assert_eq!(rows.len(), 3);
    for (row, expected) in rows[1..].iter().zip(events) {
        assert_eq!(
            serde_json::from_str::<RecordedEvent>(row)
                .expect("event")
                .event,
            expected
        );
    }
    assert!(
        Recorder::create(&path, &header(root.path())).is_err(),
        "never overwrite a session"
    );
}

#[test]
fn replay_preserves_keys_and_reports_truncated_sessions() {
    let root = TempDir::new().expect("temporary directory");
    let path = root.path().join("events.jsonl");
    let event = Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    let record = RecordedEvent {
        delay_ms: 0,
        event: event.clone(),
    };
    std::fs::write(
        &path,
        format!(
            "{}\n{}\n",
            serde_json::to_string(&header(root.path())).expect("header"),
            serde_json::to_string(&record).expect("event")
        ),
    )
    .expect("recording");
    let mut replay = Replay::load(&path, (80, 24)).expect("replay");
    assert_eq!(replay.read().expect("read"), Some(event));
    assert_eq!(
        replay.read().expect_err("EOF").kind(),
        io::ErrorKind::UnexpectedEof
    );
    assert!(Replay::load(&path, (40, 12)).is_err());
    assert!(Replay::load(&path, (120, 40)).is_ok());
}

#[test]
fn replay_requires_room_for_every_recorded_resize() {
    let root = TempDir::new().expect("directory");
    let script = root.path().join("session.py");
    let data = root.path().join("events.jsonl");
    let mut recorder = Recorder::create(&script, &header(root.path())).expect("recorder");
    recorder.record(&Event::Resize(120, 40)).expect("resize");
    replay_file(&script, &data);
    assert!(Replay::load(&data, (80, 24)).is_err());
    let input = Input::Replay(Replay::load(&data, (120, 40)).expect("replay"));
    assert_eq!(
        input.terminal_options().viewport,
        ratatui::Viewport::Fixed(ratatui::layout::Rect::new(0, 0, 80, 24))
    );
}

#[test]
fn nonblocking_lookahead_does_not_consume_recorded_events() {
    let root = TempDir::new().expect("directory");
    let script = root.path().join("session.py");
    let data = root.path().join("events.jsonl");
    let mut recorder = Recorder::create(&script, &header(root.path())).expect("recorder");
    let event = Event::Paste("Hello мир🙂".to_owned());
    recorder.record(&event).expect("paste");
    replay_file(&script, &data);
    let mut input = Input::Replay(Replay::load(&data, (80, 24)).expect("replay"));
    assert_eq!(input.read_pending().unwrap(), None);
    // What the session threw away was never recorded; what was, stays.
    assert!(!input.discard_queued().unwrap());
    assert_eq!(input.read().unwrap(), Some(event));
    assert_eq!(input.read_pending().unwrap(), None);
    assert_eq!(
        input.read().unwrap_err().kind(),
        io::ErrorKind::UnexpectedEof
    );
}

fn record_directory_workflow(recorder: &mut Recorder) {
    for code in [
        KeyCode::F(7),
        KeyCode::Char('x'),
        KeyCode::Enter,
        KeyCode::F(10),
        KeyCode::Enter,
    ] {
        recorder
            .record(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)))
            .expect("record key");
    }
}

/// Without a terminal nothing is queued, and nothing is waited for.
#[test]
fn discarding_live_input_without_a_terminal_finds_no_cancel() {
    assert!(!Input::Live(None)
        .discard_queued()
        .expect("no terminal to lose"));
}
