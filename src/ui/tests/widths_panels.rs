//! Unit tests: a panel beside double-width names keeps its Size column, and
//! only the active panel's info row carries the status message.

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::render_widths::{blank_app, left_panel_text, row_text};
use super::*;
use crate::models::DirEntry;
use crate::models::EntryKind;
use crate::models::FileSize;
use crate::models::Modified;
use crate::status::Status;

/// A CJK name is 2 cells per char. Budgeted in chars it overruns the name
/// column and pushes the size span off the row entirely.
#[test]
fn test_panel_keeps_size_visible_beside_a_double_width_name() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.left_panel.entries = vec![DirEntry {
        // 10 chars / 20 cells against a 25-cell name column.
        name: "文档档案资料备份目录".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(4096),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    }];
    app.left_panel.select(Some(0));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let panel = left_panel_text(&terminal);
    assert!(
        panel.contains("4096"),
        "the size column must survive a double-width name, got:\n{panel}"
    );
}

/// Even a short CJK name used to clip the size to "4.00": the name span
/// over-spent its allotment by one cell per glyph.
#[test]
fn test_panel_does_not_clip_size_after_a_short_double_width_name() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.left_panel.entries = vec![DirEntry {
        name: "文档.txt".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(4096),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    }];
    app.left_panel.select(Some(0));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let panel = left_panel_text(&terminal);
    assert!(panel.contains("4096"), "got:\n{panel}");
}

/// The status message is the ONLY channel a failed copy/move/view has — no
/// `log_error` call exists in the handlers. At 80 columns the fixed hint
/// prefix plus the selected-entry info filled the row exactly, so every such
/// message was clipped off-screen and silently lost.
#[test]
fn test_status_message_reaches_the_active_panels_info_row() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.left_panel.entries = vec![DirEntry {
        name: "secret.txt".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(4096),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    }];
    app.left_panel.select(Some(0));
    app.set_status(Status::CommandOutput(
        "Copied 0 item(s), 1 error(s): /srv/x/secret.txt: Permission denied (os error 13)"
            .to_owned(),
    ));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Row 20 on a 24-row screen: the two chrome rows take 22 and 23, the
    // panel's bottom border takes 21, and the info row sits directly above it.
    let info_row = row_text(&terminal, 20);
    assert!(
        info_row.contains("Copied 0 item(s), 1 error(s)"),
        "the error must reach the user, got: '{info_row}'"
    );
}

/// The message displaces the entry name, and only on the panel being acted on.
///
/// Far has no status bar, so this row is the only one left to carry a message —
/// which makes "does the *inactive* panel still name its own entry" a real
/// question rather than a detail.
#[test]
fn test_only_the_active_panel_gives_its_info_row_to_the_message() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.right_panel.entries = vec![DirEntry {
        name: "untouched.txt".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(1),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    }];
    app.right_panel.select(Some(0));
    app.set_status(Status::CommandOutput("busy".to_owned()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let info_row = row_text(&terminal, 20);
    assert!(info_row.contains("busy"), "got: '{info_row}'");
    assert!(
        info_row.contains("untouched.txt"),
        "the inactive panel keeps naming its own entry, got: '{info_row}'"
    );
}
