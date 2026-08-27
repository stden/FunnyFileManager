//! Unit tests: column widths and scroll offsets on a default 80x24 terminal.
//!
//! Everything here renders through the real `ui`/`render_overlays` path — the
//! bugs these pin were invisible to tests that only asserted "does not panic"
//! or that used an artificially wide terminal.
#![allow(unused_imports)]

use super::*;
use crate::models::CharColumn;
use crate::models::EntryKind;
use crate::models::FileSize;
use crate::scroll::ScrollTop;
use crate::status::Status;
use crate::ui::Cells;

use crate::models::{App, DirEntry, Editor, Overlay, Panel, Viewer};

use ratatui::backend::TestBackend;

use ratatui::Terminal;

use std::path::PathBuf;

use tempfile::TempDir;

/// One buffer row as a string, trailing spaces trimmed.
fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buf = terminal.backend().buffer();
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol())
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// Column of the caret on `y`, found by its reversed background.
///
/// The caret is a *style* now, not a `_` glyph: it has to be legible sitting on
/// a character as well as after the last one. So "the cursor is on the row" is
/// a question about which cell is painted, and `row_text` — which trims
/// trailing blanks — cannot answer it at the end of a line.
fn caret_column(terminal: &Terminal<TestBackend>, y: u16) -> Option<u16> {
    let buf = terminal.backend().buffer();
    (0..buf.area.width).find(|&x| buf[(x, y)].bg == Color::Yellow)
}

/// The whole screen as one string.
fn screen_text(terminal: &Terminal<TestBackend>) -> String {
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
fn left_panel_text(terminal: &Terminal<TestBackend>) -> String {
    let area = terminal.backend().buffer().area;
    let half = area.width / 2;
    (0..area.height)
        .map(|y| {
            let buf = terminal.backend().buffer();
            (0..half)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
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
fn blank_app() -> (App, TempDir) {
    let temp = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp.path().to_path_buf()).expect("test setup");
    app.left_panel.entries.clear();
    app.right_panel.entries.clear();
    (app, temp)
}

#[test]
fn test_fit_name_pads_short_and_ellipsises_long_names() {
    assert_eq!(fit_name("a.txt", Cells::new(8)), "a.txt   ");
    assert_eq!(fit_name("abcdefgh", Cells::new(8)), "abcdefgh");
    assert_eq!(fit_name("abcdefghij", Cells::new(8)), "abcdefg…");
    // Cut by chars, not bytes: a byte cut would split the Cyrillic name.
    assert_eq!(fit_name("файл-очень-длинный", Cells::new(6)), "файл-…");
    assert_eq!(fit_name("файл", Cells::new(0)), "");
}

#[test]
fn test_tail_cells_keeps_the_end_of_the_path() {
    assert_eq!(tail_cells("/home/user", Cells::new(20)), "/home/user");
    assert_eq!(tail_cells("/home/user/deep", Cells::new(6)), "…/deep");
    assert_eq!(tail_cells("/дом/юзер", Cells::new(5)), "…юзер");
    assert_eq!(tail_cells("/home", Cells::new(0)), "");
}

/// Budgets are in terminal CELLS, not chars: ratatui subtracts each grapheme's
/// `cell_width` from the row, so a CJK glyph spends two of them. Measuring in
/// chars over-fills the row and pushes the span that follows off the screen.
#[test]
fn test_width_helpers_budget_in_cells_not_chars() {
    // 5 chars, 10 cells — a char-budgeted `fit_name` would return it unchanged
    // and blow the 8-column allotment by two.
    assert_eq!(fit_name("文档档案资", Cells::new(8)), "文档档… ");
    // 2 chars / 4 cells into a 6-cell column: two spaces of padding, not four.
    assert_eq!(fit_name("文档", Cells::new(6)), "文档  ");
    // A double-width glyph is dropped whole rather than half-drawn, so the
    // leftover cell is padded on the right and the size column stays aligned.
    assert_eq!(fit_name("文档档案", Cells::new(6)), "文档… ");
    // Same rule cutting a path from the head.
    assert_eq!(tail_cells("/文档/资料", Cells::new(5)), "…资料");
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
            name: "docs".to_string(),
            kind: EntryKind::Directory,
            size: FileSize::from_bytes(0),
        },
        DirEntry {
            name: "image.jpg".to_string(),
            kind: EntryKind::File,
            size: FileSize::from_bytes(1_572_864), // 1.50 MB
        },
    ];
    app.left_panel.select(Some(0));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let panel = left_panel_text(&terminal);
    assert!(panel.contains("<DIR>"), "got:\n{panel}");
    assert!(panel.contains("1.50 MB"), "got:\n{panel}");
}

/// A name longer than the panel must be cut, not allowed to push the size out.
#[test]
fn test_panel_truncates_long_name_and_keeps_size_visible() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.left_panel.entries = vec![DirEntry {
        name: "очень-длинное-имя-файла-которое-не-помещается.txt".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(4096),
    }];
    app.left_panel.select(Some(0));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let panel = left_panel_text(&terminal);
    assert!(
        panel.contains('…'),
        "long name should be cut, got:\n{panel}"
    );
    assert!(panel.contains("4.00 KB"), "got:\n{panel}");
}

/// A deep path must not push the typed command and cursor off the row.
#[test]
fn test_command_line_keeps_typed_text_visible_on_long_path() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.left_panel.path = PathBuf::from("/".to_string() + &vec!["component"; 30].join("/"));
    app.set_overlay(Overlay::CommandLine("ls".into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd_row = row_text(&terminal, 22);
    assert!(cmd_row.ends_with("> ls"), "got: '{cmd_row}'");
    assert!(
        cmd_row.starts_with('…'),
        "path should be cut, got: '{cmd_row}'"
    );
    // The caret sits one cell past the typed text, still inside the row.
    assert_eq!(
        caret_column(&terminal, 22),
        Some(u16::try_from(cmd_row.chars().count()).expect("row fits u16")),
        "the caret must stay on the row, got: '{cmd_row}'"
    );
}

/// A CJK name is 2 cells per char. Budgeted in chars it overruns the name
/// column and pushes the size span off the row entirely.
#[test]
fn test_panel_keeps_size_visible_beside_a_double_width_name() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.left_panel.entries = vec![DirEntry {
        // 10 chars / 20 cells against a 25-cell name column.
        name: "文档档案资料备份目录".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(4096),
    }];
    app.left_panel.select(Some(0));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let panel = left_panel_text(&terminal);
    assert!(
        panel.contains("4.00 KB"),
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
        name: "文档.txt".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(4096),
    }];
    app.left_panel.select(Some(0));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let panel = left_panel_text(&terminal);
    assert!(panel.contains("4.00 KB"), "got:\n{panel}");
}

/// A deep CJK path must not push the prompt, typed text and cursor off the row.
#[test]
fn test_command_line_survives_a_double_width_path() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.left_panel.path = PathBuf::from("/".to_string() + &vec!["文档目录"; 30].join("/"));
    app.set_overlay(Overlay::CommandLine("ls".into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd_row = row_text(&terminal, 22);
    assert!(cmd_row.ends_with("> ls"), "got: '{cmd_row}'");
    assert!(
        caret_column(&terminal, 22).is_some(),
        "the caret must stay on the row, got: '{cmd_row}'"
    );
}

/// The status message is the ONLY channel a failed copy/move/view has — no
/// `log_error` call exists in the handlers. At 80 columns the fixed hint
/// prefix plus the selected-entry info filled the row exactly, so every such
/// message was clipped off-screen and silently lost.
#[test]
fn test_status_message_wins_the_row_over_the_key_hints() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.left_panel.entries = vec![DirEntry {
        name: "secret.txt".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(4096),
    }];
    app.left_panel.select(Some(0));
    app.set_status(Status::CommandOutput(
        "Copied 0 item(s), 1 error(s): /srv/x/secret.txt: Permission denied (os error 13)"
            .to_string(),
    ));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let status_row = row_text(&terminal, 21);
    assert!(
        status_row.contains("Permission denied (os error 13)"),
        "the error must reach the user, got: '{status_row}'"
    );
    assert!(
        status_row.contains("Copied 0 item(s), 1 error(s)"),
        "the whole message should fit at 80 columns, got: '{status_row}'"
    );
}

/// A command longer than the terminal is wide must keep its tail and cursor:
/// the typed text is the one thing the user cannot read anywhere else.
#[test]
fn test_command_line_windows_a_command_longer_than_the_row() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.left_panel.path = PathBuf::from("/tmp");
    let command = "cd ".to_string() + &"a".repeat(117);
    app.set_overlay(Overlay::CommandLine(command.as_str().into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd_row = row_text(&terminal, 22);
    assert!(
        cmd_row.ends_with('a'),
        "the command tail must stay on the row, got: '{cmd_row}'"
    );
    assert_eq!(
        caret_column(&terminal, 22),
        Some(u16::try_from(cmd_row.chars().count()).expect("row fits u16")),
        "the caret must stay on the row, got: '{cmd_row}'"
    );
    assert!(
        cmd_row.starts_with("> …"),
        "the command should be windowed from its head, got: '{cmd_row}'"
    );
}

/// A narrow terminal must still show file names. The fixed 8-column size field
/// used to have absolute priority, leaving the name column zero cells wide.
#[test]
fn test_narrow_terminal_still_renders_file_names() {
    let backend = TestBackend::new(30, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.left_panel.entries = vec![DirEntry {
        name: "notes.md".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(4096),
    }];
    app.left_panel.select(Some(0));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let panel = left_panel_text(&terminal);
    assert!(
        panel.contains("notes.md"),
        "a 30-column panel must still name its files, got:\n{panel}"
    );
}

/// Horizontal editor scrolling past u16 must not wrap around: a minified
/// one-line file is well under the edit size limit and easily 200k chars wide.
#[test]
fn test_editor_scrolls_past_u16_without_wrapping() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    let content = "x".repeat(70_000) + "MARKER";
    let mut editor =
        Editor::from_content("bundle.min.js".to_string(), PathBuf::from("b.js"), &content);
    editor.move_end();
    app.set_overlay(Overlay::Editor(editor));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let screen = screen_text(&terminal);
    assert!(
        screen.contains("MARKER"),
        "editor must show the text under the cursor, got:\n{screen}"
    );
}

/// The editor overlay is the third site of the chars-vs-cells rule: the model's
/// `cursor_col` is a char index (right for editing), but the horizontal window
/// and the cursor land on screen COLUMNS. On a CJK line a char-indexed window
/// scrolls half as far as it should and the block cursor sits under the wrong
/// glyph.
#[test]
fn test_editor_overlay_scrolls_and_places_the_cursor_in_cells() {
    let backend = TestBackend::new(30, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    // 40 CJK chars = 80 cells, far wider than the ~25-cell editor window.
    let content = "文".repeat(40) + "标记";
    let mut editor =
        Editor::from_content("cjk.txt".to_string(), PathBuf::from("cjk.txt"), &content);
    editor.move_end();
    app.set_overlay(Overlay::Editor(editor));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // A wide glyph occupies two buffer cells and the second renders blank, so
    // assert on the glyphs rather than on an exact substring.
    let screen = screen_text(&terminal);
    assert!(
        screen.contains('标') && screen.contains('记'),
        "the window must scroll far enough to show the text under the cursor, got:\n{screen}"
    );

    // The cursor sits one cell past "记", at the last inner column: budgeting in
    // chars would have halved the scroll and parked it mid-line instead.
    let area = centered_rect(90, 90, terminal.backend().buffer().area);
    assert_eq!(
        terminal.get_cursor_position().unwrap().x,
        area.x + area.width - 2,
        "the cursor must follow the double-width text, not its char count"
    );
}

/// Same wraparound for the viewer's vertical offset.
#[test]
fn test_viewer_scrolls_past_u16_without_wrapping() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    let mut lines: Vec<String> = (0..70_000).map(|i| format!("line {i}")).collect();
    lines[69_000] = "MARKER".to_string();
    app.set_overlay(Overlay::Viewer(Viewer {
        title: "huge.log".to_string(),
        lines,
        scroll: ScrollTop::TOP.down(69_000, usize::MAX),
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let screen = screen_text(&terminal);
    assert!(
        screen.contains("MARKER"),
        "viewer must show the scrolled-to line, got:\n{screen}"
    );
}

// ── Cells ─────────────────────────────────────────────────────────────────

#[test]
fn cells_measures_columns_not_characters() {
    // The distinction the newtype exists to enforce: a CJK glyph is one char
    // and two columns, and every budget on a row is spent in columns.
    assert_eq!(Cells::of("abc").get(), 3);
    assert_eq!(Cells::of("漢字").get(), 4);
    assert_eq!("漢字".chars().count(), 2);
    assert_eq!(Cells::of("").get(), 0);
    assert!(Cells::of("").is_zero());
}

#[test]
fn cells_arithmetic_saturates_instead_of_wrapping() {
    // Every budget in this module is "what is left after"; underflowing it
    // would wrap to a colossal width and draw past the terminal edge.
    assert_eq!(Cells::new(3).saturating_sub(Cells::new(10)), Cells::ZERO);
    assert_eq!(Cells::new(10).saturating_sub(Cells::new(3)).get(), 7);
    assert_eq!((Cells::new(2) + Cells::new(3)).get(), 5);
}

#[test]
fn cells_before_converts_a_char_index_to_a_column() {
    // The one place chars and columns legitimately meet: the editor's cursor
    // is char-indexed, the screen is column-indexed.
    assert_eq!(cells_before("漢字abc", CharColumn::new(2)), Cells::new(4));
    assert_eq!(cells_before("abc", CharColumn::new(2)), Cells::new(2));
    assert_eq!(cells_before("abc", CharColumn::new(99)), Cells::new(3));
}
