//! Purpose: `Ctrl+Q` through the real event loop — the passive panel previews
//! the file under the cursor and follows it as the cursor moves.
#![cfg(feature = "test-helpers")]

use std::fs;
use std::io;
use std::path::Path;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::models::{App, Panel};
use funny_file_manager::{run_app_with_event_reader, WritableTestBackend};
use ratatui::Terminal;
use tempfile::TempDir;

fn screen(terminal: &Terminal<WritableTestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    let width = usize::from(buffer.area.width);
    buffer
        .content
        .iter()
        .map(|cell| cell.symbol().to_owned())
        .collect::<Vec<_>>()
        .chunks(width)
        .map(<[String]>::concat)
        .collect::<Vec<_>>()
        .join("\n")
}

#[expect(
    clippy::expect_used,
    reason = "test fixture: a viewport that cannot be built fails the test that asked for it"
)]
fn terminal_for_preview(path: &Path) -> Terminal<WritableTestBackend> {
    // Keep the complete preview title visible even with long Windows temp paths.
    let columns = u16::try_from(path.to_string_lossy().len() * 2 + 12).expect("test viewport");
    Terminal::new(WritableTestBackend::new(columns.max(100), 24)).expect("test terminal")
}

#[test]
fn the_preview_follows_the_cursor_through_the_loop() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("a_first.txt"), "FIRST-FILE-BODY\n").expect("write");
    fs::write(dir.path().join("b_second.txt"), "SECOND-FILE-BODY\n").expect("write");
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    let first = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == "a_first.txt");
    app.left_panel.select(first);

    let mut terminal = terminal_for_preview(&dir.path().join("b_second.txt"));
    let mut events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL)),
        Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty())),
        Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
    ]
    .into_iter();
    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "no more events"))
    })
    .expect("event loop");

    let shown = screen(&terminal);
    assert!(shown.contains("SECOND-FILE-BODY"), "{shown}");
    assert!(!shown.contains("FIRST-FILE-BODY"), "{shown}");
    assert!(shown.contains("b_second.txt"), "{shown}");
}

#[test]
fn a_folder_readme_is_rendered_through_the_loop() {
    let dir = TempDir::new().expect("temp dir");
    let folder = dir.path().join("project");
    fs::create_dir(&folder).expect("folder");
    fs::write(
        folder.join("README.md"),
        "# PROJECT-NOTES\nFolder description\n",
    )
    .expect("readme");
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    let selected = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == "project");
    app.left_panel.select(selected);
    let mut terminal = terminal_for_preview(&folder);
    let mut keys = ['q', 'c'].into_iter();
    run_app_with_event_reader(&mut terminal, app, || {
        keys.next()
            .map(|key| Event::Key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::CONTROL)))
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "no more keys"))
    })
    .expect("event loop");

    let shown = screen(&terminal);
    assert!(shown.contains("PROJECT-NOTES"), "{shown}");
    assert!(shown.contains("Folder description"), "{shown}");
    assert!(!shown.contains("Total size:"), "{shown}");
}
