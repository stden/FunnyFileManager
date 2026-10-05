//! Unit tests: a panel's name and Size columns on a default 80x24 terminal and
//! on narrow ones, plus the screen helpers the `widths_*` tests share.
//!
//! Everything here renders through the real `ui` path — the bugs these pin
//! were invisible to tests that only asserted "does not panic" or that used
//! an artificially wide terminal.

use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tempfile::TempDir;

use super::*;
use crate::models::EntryKind;
use crate::models::FileSize;
use crate::models::Modified;
use crate::models::{DirEntry, Panel};

/// One buffer row as a string, trailing spaces trimmed.
pub(super) fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buf = terminal.backend().buffer();
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol())
        .collect::<String>()
        .trim_end()
        .to_owned()
}

/// Column of the caret on `y`, found by its reversed background.
///
/// The caret is a *style* now, not a `_` glyph: it has to be legible sitting on
/// a character as well as after the last one. So "the cursor is on the row" is
/// a question about which cell is painted, and `row_text` — which trims
/// trailing blanks — cannot answer it at the end of a line.
pub(super) fn caret_column(terminal: &Terminal<TestBackend>, y: u16) -> Option<u16> {
    let buf = terminal.backend().buffer();
    (0..buf.area.width).find(|&x| buf[(x, y)].bg == Color::Yellow)
}

/// The whole screen as one string.
pub(super) fn screen_text(terminal: &Terminal<TestBackend>) -> String {
    let height = terminal.backend().buffer().area.height;
    (0..height)
        .map(|y| row_text(terminal, y))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Only the columns the LEFT panel owns — the panels split the width 50/50.
///
/// A whole-screen `contains` cannot tell which panel answered it, and the right
/// panel is never the one under test here: every `<DIR>`/`…`/size assertion
/// below used to be satisfiable by the right panel's own listing.
pub(super) fn left_panel_text(terminal: &Terminal<TestBackend>) -> String {
    let area = terminal.backend().buffer().area;
    let half = area.width / 2;
    (0..area.height)
        .map(|y| {
            let buf = terminal.backend().buffer();
            (0..half)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// An `App` whose panels show nothing at all.
///
/// `App::new()` points BOTH panels at the process's cwd, and its listing starts
/// with `..` — which renders as `<DIR>` — followed by whatever the repository
/// happens to contain. Screen-wide assertions were therefore answered by the
/// environment: `test_panel_shows_full_size_column_at_80_columns` stayed green
/// with `left_panel.entries` emptied. Starting from blank panels means anything
/// found on screen was put there by the test itself. The `TempDir` is returned
/// because it must outlive the render.
pub(super) fn blank_app() -> (App, TempDir) {
    let temp = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp.path().to_path_buf()).expect("test setup");
    app.left_panel.entries.clear();
    app.right_panel.entries.clear();
    (app, temp)
}

/// The size column must survive on the default 80-column terminal, where each
/// panel is only 40 wide.
#[test]
fn test_panel_shows_full_size_column_at_80_columns() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.left_panel.entries = vec![
        DirEntry {
            name: "docs".to_owned(),
            kind: EntryKind::Directory,
            size: FileSize::from_bytes(0),
            modified: Modified::UNKNOWN,
            executable: false,
            ..DirEntry::default()
        },
        DirEntry {
            name: "image.jpg".to_owned(),
            kind: EntryKind::File,
            size: FileSize::from_bytes(1_572_864), // 1.50 MB,
            modified: Modified::UNKNOWN,
            executable: false,
            ..DirEntry::default()
        },
    ];
    app.left_panel.select(Some(0));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let panel = left_panel_text(&terminal);
    // Far names a directory rather than measuring it, and "Folder" fills the
    // six-cell column exactly — no leading blank in front of the word.
    assert!(panel.contains("│Folder│"), "got:\n{panel}");
    // 1572864 needs seven digits and the column holds six, so it abbreviates
    // rather than being clipped to a wrong number.
    assert!(panel.contains("1.5 M"), "got:\n{panel}");
}

/// A name longer than the panel must be cut, not allowed to push the size out.
#[test]
fn test_panel_truncates_long_name_and_keeps_size_visible() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.left_panel.entries = vec![DirEntry {
        name: "очень-длинное-имя-файла-которое-не-помещается.txt".to_owned(),
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
        panel.contains('}'),
        "long name should be cut, got:\n{panel}"
    );
    assert!(panel.contains("4096"), "got:\n{panel}");
}

/// A narrow terminal must still show file names. The fixed 8-column size field
/// used to have absolute priority, leaving the name column zero cells wide.
#[test]
fn test_narrow_terminal_still_renders_file_names() {
    let backend = TestBackend::new(30, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.left_panel.entries = vec![DirEntry {
        name: "notes.md".to_owned(),
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
        panel.contains("notes.md"),
        "a 30-column panel must still name its files, got:\n{panel}"
    );
}
