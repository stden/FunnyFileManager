//! Opening a document invokes the OS handler with the selected absolute path.

#![cfg(target_os = "linux")]

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::models::{App, Panel};
use funny_file_manager::test_support::EnvVarGuard;
use funny_file_manager::{run_app_with_event_reader, WritableTestBackend};
use ratatui::Terminal;

#[test]
fn opening_a_document_passes_its_path_to_the_desktop_handler() {
    let temporary = tempfile::TempDir::new().expect("temporary workspace");
    let document_dir = temporary.path().join("documents");
    let handler_dir = temporary.path().join("bin");
    fs::create_dir(&document_dir).expect("document directory");
    fs::create_dir(&handler_dir).expect("handler directory");
    let document = document_dir.join("hello world.txt");
    fs::write(&document, "test document").expect("document");
    let handler = handler_dir.join("xdg-open");
    fs::write(
        &handler,
        "#!/bin/sh\nprintf '%s' \"$1\" > \"${0%/*}/opened\"\n",
    )
    .expect("fake handler");
    fs::set_permissions(&handler, fs::Permissions::from_mode(0o755)).expect("executable");
    let _path = EnvVarGuard::set("PATH", &handler_dir);
    let panel = Panel::new(document_dir).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    app.config.lsp.0.clear();
    let mut events = [KeyCode::Down, KeyCode::Enter].into_iter();
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .map(|code| Event::Key(KeyEvent::new(code, KeyModifiers::NONE)))
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "test complete"))
    });
    assert!(result.is_err());
    assert_eq!(
        fs::read_to_string(handler_dir.join("opened")).expect("handler invocation"),
        document.to_string_lossy()
    );
}
