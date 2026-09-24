//! Purpose: A failed recording must stop safely and report its failure exactly once.

use super::*;
use std::fs::File;
use std::time::Instant;
use tempfile::TempDir;

#[test]
fn losing_write_access_stops_recording_without_ending_the_input_session() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("read-only-recording");
    std::fs::write(&path, "kept").unwrap();
    let recorder = Recorder {
        file: File::open(&path).unwrap(),
        previous: Instant::now(),
    };
    let mut active = Some(Rc::new(RefCell::new(recorder)));
    assert!(stopped_reason().is_none());
    record_live(&mut active, &Event::Resize(80, 24));
    assert!(active.is_none());
    assert!(stopped_reason().is_some());
    assert!(
        stopped_reason().is_none(),
        "failure is reported exactly once"
    );
    record_live(&mut active, &Event::FocusLost);
    assert!(stopped_reason().is_none());
    assert_eq!(std::fs::read_to_string(path).unwrap(), "kept");
}

#[test]
fn successful_live_recording_keeps_the_recorder_and_preserves_the_input() {
    let root = TempDir::new().unwrap();
    let script = root.path().join("session.py");
    let mut input = Input::open_recorder(&script, (80, 24)).unwrap();
    let Input::Live(active) = &mut input else {
        panic!()
    };
    let event = Event::Paste("recorded text".into());
    record_live(active, &event);
    assert!(active.is_some());
    let source = std::fs::read_to_string(script).unwrap();
    let last = source
        .lines()
        .last()
        .unwrap()
        .strip_prefix("# FFM ")
        .unwrap();
    let recorded: RecordedEvent = serde_json::from_str(last).unwrap();
    assert_eq!(recorded.event, event);
    assert!(stopped_reason().is_none());
}
