//! Recordings preserve input payloads without generating Python expressions.
use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

struct TextClipboard(&'static str);

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

impl crate::clipboard::ClipboardBackend for TextClipboard {
    fn get_native(&mut self) -> Result<String, String> {
        Ok(self.0.to_owned())
    }
    fn set_native(&mut self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn emit_osc52(&mut self, _: &str) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn replay_pastes_recorded_clipboard_text_instead_of_the_current_clipboard() {
    let root = TempDir::new().expect("directory");
    let script = root.path().join("session.py");
    let input = Input::open_recorder(script.clone(), (80, 24)).expect("record");
    let mut clipboard = input.clipboard(Box::new(TextClipboard("recorded Ж🙂")));
    assert_eq!(clipboard.get_native().expect("read"), "recorded Ж🙂");
    let json = std::fs::read_to_string(script)
        .expect("script")
        .lines()
        .filter_map(|line| line.strip_prefix("# FFM "))
        .collect::<Vec<_>>()
        .join("\n");
    let path = root.path().join("events.jsonl");
    std::fs::write(&path, json).expect("recording");
    let replay = Input::Replay(Replay::load(&path, (80, 24)).expect("load"));
    let mut clipboard = replay.clipboard(Box::new(TextClipboard("changed clipboard")));
    assert_eq!(
        clipboard.get_native().expect("replayed read"),
        "recorded Ж🙂"
    );
    assert!(
        clipboard.get_native().is_err(),
        "missing recorded reads must not use live text"
    );
}

#[test]
fn live_clipboard_writes_reach_the_host_backend() {
    let root = TempDir::new().expect("directory");
    let recorded = Input::open_recorder(root.path().join("session.py"), (80, 24)).expect("record");
    let mut clipboard = recorded.clipboard(Box::new(TextClipboard("host")));
    assert!(clipboard.set_native("copied").is_ok());
    assert!(clipboard.emit_osc52("copied").is_ok());

    let mut passthrough = Input::Live(None).clipboard(Box::new(TextClipboard("host")));
    assert_eq!(passthrough.get_native().expect("inner"), "host");
    assert!(passthrough.set_native("x").is_ok());
    assert!(passthrough.emit_osc52("x").is_ok());
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
