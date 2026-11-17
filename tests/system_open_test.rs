//! Opening a document invokes the OS handler with the selected absolute path.

#![cfg(any(target_os = "linux", windows))]

use std::fs;
use std::io;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::models::{App, Panel};
#[cfg(target_os = "linux")]
use funny_file_manager::test_support::EnvVarGuard;
use funny_file_manager::{run_app_with_event_reader, WritableTestBackend};
use ratatui::Terminal;

#[expect(clippy::expect_used, reason = "test fixture terminal")]
fn open_first_entry(panel: Panel) -> Terminal<WritableTestBackend> {
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
    terminal
}

fn screen_text(terminal: &Terminal<WritableTestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

#[cfg(target_os = "linux")]
#[test]
fn opening_a_document_passes_its_path_and_reports_handler_failure() {
    use std::os::unix::fs::PermissionsExt;

    let temporary = tempfile::TempDir::new().expect("temporary workspace");
    let document_dir = temporary.path().join("documents");
    let handler_dir = temporary.path().join("bin");
    fs::create_dir(&document_dir).expect("document directory");
    fs::create_dir(&handler_dir).expect("handler directory");
    let document = document_dir.join("hello world.txt");
    fs::write(&document, "test document").expect("document");
    let handler = handler_dir.join("xdg-open");
    let script = "#!/bin/sh\nprintf '%s' \"$1\" > \"${0%/*}/opened\"\n";
    fs::write(&handler, script).expect("fake handler");
    fs::set_permissions(&handler, fs::Permissions::from_mode(0o755)).expect("executable");
    let _path = EnvVarGuard::set("PATH", &handler_dir);
    let panel = Panel::new(document_dir).expect("panel");
    drop(open_first_entry(panel.clone()));
    let opened = fs::read_to_string(handler_dir.join("opened")).expect("handler invocation");
    assert_eq!(opened, document.to_string_lossy());

    fs::write(&handler, "#!/bin/sh\nexit 1\n").expect("failing handler");
    assert!(screen_text(&open_first_entry(panel)).contains("Cannot run"));
}

#[cfg(windows)]
#[test]
fn opening_a_missing_unassociated_document_reports_host_association_failure() {
    let temporary = tempfile::TempDir::new().expect("temporary workspace");
    let document_dir = temporary.path().join("documents");
    fs::create_dir(&document_dir).expect("document directory");
    let document = document_dir.join("missing.ffm-unassociated-ext");
    fs::write(&document, "test document").expect("document");
    let panel = Panel::new(document_dir).expect("panel");
    fs::remove_file(&document).expect("remove before open");
    let terminal = open_first_entry(panel);
    let screen = screen_text(&terminal);
    assert!(screen.contains("Cannot run"), "{screen}");
}
