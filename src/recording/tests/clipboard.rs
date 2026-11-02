//! Purpose: clipboard reads are recorded and replayed in place of the host clipboard; writes still reach the host.

use super::*;

struct TextClipboard(&'static str);

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
    let input = Input::open_recorder(&script, (80, 24)).expect("record");
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
    let recorded = Input::open_recorder(&root.path().join("session.py"), (80, 24)).expect("record");
    let mut clipboard = recorded.clipboard(Box::new(TextClipboard("host")));
    assert!(clipboard.set_native("copied").is_ok());
    assert!(clipboard.emit_osc52("copied").is_ok());

    let mut passthrough = Input::Live(None).clipboard(Box::new(TextClipboard("host")));
    assert_eq!(passthrough.get_native().expect("inner"), "host");
    assert!(passthrough.set_native("x").is_ok());
    assert!(passthrough.emit_osc52("x").is_ok());
}
