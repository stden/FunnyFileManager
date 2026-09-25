//! Unit tests: keys typed while one event kept the loop busy are thrown away,
//! and keys queued behind a quick one still run.

use std::collections::VecDeque;
use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::*;
use crate::models::Panel;

/// A terminal whose every event is already typed: `read` hands them out one
/// by one, and so does `try_read`, which is how the loop sees what is queued.
struct Typed(VecDeque<Event>);

impl EventSource for Typed {
    fn read(&mut self) -> io::Result<Option<Event>> {
        self.0
            .pop_front()
            .map(Some)
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "nothing more typed"))
    }

    fn try_read(&mut self) -> io::Result<Option<Event>> {
        Ok(self.0.pop_front())
    }
}

/// A panel on a folder holding `a-keep.txt`, the cursor on it, and the
/// command line holding `command`.
fn app_on_keep_file(folder: &Path, command: &str) -> App {
    fs::write(folder.join("a-keep.txt"), b"precious").expect("test setup");
    let panel = || Panel::new(folder.to_path_buf()).expect("test setup");
    let mut app = App::with_panels(panel(), panel());
    app.left_panel.select_named("a-keep.txt");
    if !command.is_empty() {
        app.set_overlay(Overlay::CommandLine(command.into()));
    }
    app
}

fn run(app: &mut App, keys: &[KeyCode]) {
    let typed = Typed(
        keys.iter()
            .map(|&code| Event::Key(key_event(code)))
            .collect(),
    );
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("test setup");
    let backend: Arc<dyn crate::wsl::WslBackend> = Arc::new(crate::wsl::SystemWslBackend);
    let ended = run_loop(&mut terminal, app, typed, backend);
    let error = ended.expect_err("the typed keys run out before anything quits");
    assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof, "{error}");
}

/// The guest sweep's data loss: F8 Enter typed at a frozen screen deleted a
/// file once the long operation behind it was over.
#[cfg(unix)]
#[test]
fn keys_typed_during_a_long_operation_never_reach_the_panel() {
    let folder = TempDir::new().expect("test setup");
    let mut app = app_on_keep_file(folder.path(), "sleep 0.6");

    run(&mut app, &[KeyCode::Enter, KeyCode::F(8), KeyCode::Enter]);

    assert_eq!(
        fs::read(folder.path().join("a-keep.txt")).expect("the file survives"),
        b"precious"
    );
    assert!(
        matches!(app.overlay, Overlay::None),
        "F8 opened nothing either"
    );
}

/// The same keys behind a quick event are the user's next command.
#[test]
fn keys_queued_behind_a_quick_event_still_run() {
    let folder = TempDir::new().expect("test setup");
    let mut app = app_on_keep_file(folder.path(), "");

    run(&mut app, &[KeyCode::F(8), KeyCode::Enter]);

    assert!(!folder.path().join("a-keep.txt").exists(), "{}", app.status);
}
