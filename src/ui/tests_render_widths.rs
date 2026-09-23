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
use crate::models::Modified;
use crate::models::PanelView;
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

/// Far marks a cut name with `}`, not an ellipsis — see [`clip_name`].
#[test]
fn test_clip_name_marks_cut_names_the_way_far_does() {
    assert_eq!(clip_name("abcdefgh", Cells::new(8)), "abcdefgh");
    assert_eq!(clip_name("abcdefghij", Cells::new(8)), "abcdefg}");
    // Cut by chars, not bytes: a byte cut would split the Cyrillic name.
    assert_eq!(clip_name("файл-очень-длинный", Cells::new(6)), "файл-}");
    assert_eq!(clip_name("файл", Cells::new(0)), "");
}

#[test]
fn test_pad_cells_fills_a_short_name_to_the_column_width() {
    assert_eq!(pad_cells("a.txt", Cells::new(8)), "a.txt   ");
    assert_eq!(pad_cells("abcdefgh", Cells::new(8)), "abcdefgh");
    // Already over budget: padding cannot shrink it, and must not panic.
    assert_eq!(pad_cells("abcdefghij", Cells::new(8)), "abcdefghij");
}

#[test]
fn fit_name_truncates_with_ellipsis_and_pads_to_exact_width() {
    assert_eq!(fit_name("a.txt", Cells::new(8)), "a.txt   ");
    assert_eq!(fit_name("abcdefgh", Cells::new(8)), "abcdefgh");
    assert_eq!(fit_name("abcdefghij", Cells::new(8)), "abcdefg…");
    // Zero width: nothing fits and nothing pads — no panic, no stray ellipsis.
    assert_eq!(fit_name("anything", Cells::new(0)), "");
}

#[test]
fn fit_name_handles_wide_chars() {
    // 2 chars / 4 cells into 6: two blanks of padding, not four.
    assert_eq!(fit_name("文档", Cells::new(6)), "文档  ");
    // The glyph before the ellipsis is dropped whole, so the cut lands one
    // cell short and the pad restores the exact width — a column built from
    // the cut alone drifts out of alignment here.
    assert_eq!(fit_name("文档档案", Cells::new(6)), "文档… ");
    assert_eq!(fit_name("文档档案", Cells::new(5)), "文档…");
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
    // 5 chars, 10 cells — a char-budgeted `clip_name` would return it unchanged
    // and blow the 8-column allotment by two.
    assert_eq!(clip_name("文档档案资", Cells::new(8)), "文档档}");
    // 2 chars / 4 cells into a 6-cell column: two spaces of padding, not four.
    assert_eq!(pad_cells("文档", Cells::new(6)), "文档  ");
    // A double-width glyph is dropped whole rather than half-drawn, so the
    // leftover cell is padded on the right and the size column stays aligned.
    assert_eq!(
        pad_cells(&clip_name("文档档案", Cells::new(6)), Cells::new(6)),
        "文档} "
    );
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
            modified: Modified::UNKNOWN,
            executable: false,
            ..DirEntry::default()
        },
        DirEntry {
            name: "image.jpg".to_string(),
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
        name: "очень-длинное-имя-файла-которое-не-помещается.txt".to_string(),
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
    assert!(ends_with_prompt(&cmd_row, "ls"), "got: '{cmd_row}'");
    // The path yields first and from its head — but the segments before it
    // survive, so the cut is inside the row rather than at column 0.
    assert!(
        cmd_row.contains('…'),
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
        name: "文档.txt".to_string(),
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
    assert!(ends_with_prompt(&cmd_row, "ls"), "got: '{cmd_row}'");
    let caret = caret_column(&terminal, 22).expect("painted command-line caret");
    assert_eq!(
        terminal.get_cursor_position().unwrap(),
        ratatui::layout::Position::new(caret, 22),
        "the terminal cursor must follow the cell-measured caret"
    );
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
        name: "secret.txt".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(4096),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    }];
    app.left_panel.select(Some(0));
    app.set_status(Status::CommandOutput(
        "Copied 0 item(s), 1 error(s): /srv/x/secret.txt: Permission denied (os error 13)"
            .to_string(),
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
        name: "untouched.txt".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(1),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    }];
    app.right_panel.select(Some(0));
    app.set_status(Status::CommandOutput("busy".to_string()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let info_row = row_text(&terminal, 20);
    assert!(info_row.contains("busy"), "got: '{info_row}'");
    assert!(
        info_row.contains("untouched.txt"),
        "the inactive panel keeps naming its own entry, got: '{info_row}'"
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
        cmd_row.starts_with("# …") || cmd_row.starts_with("$ …"),
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

    // The cursor sits one cell past "记", at the last column: budgeting in chars
    // would have halved the scroll and parked it mid-line instead. The editor
    // is borderless and full-screen, so "last column" is the screen's.
    //
    // One cell short of the right edge, not on it: the window starts on an odd
    // cell, and a double-width glyph straddling that cut is dropped whole
    // rather than half-drawn — so the row ends one column early and the cursor
    // goes with it.
    let area = terminal.backend().buffer().area;
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
        scroll: ScrollTop::TOP.down(69_000, usize::MAX),
        ..Viewer::report("huge.log", lines)
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

/// ratatui refuses to draw any grapheme holding a control character, so a
/// name carrying one must be measured without it. Counting it budgeted the row
/// for a column that never appeared and slid Size, Date and Time one cell left
/// of their headings for that row alone.
#[test]
fn cells_ignores_what_the_terminal_will_not_draw() {
    assert_eq!(Cells::of("tab\there").get(), Cells::of("tabhere").get());
    assert_eq!(
        Cells::of("bel\u{7}-cr\r-bs\u{8}").get(),
        Cells::of("bel-cr-bs").get()
    );
    assert_eq!(Cells::of("esc\u{1b}[31m").get(), Cells::of("esc[31m").get());
    assert_eq!(Cells::of("\u{1}\u{2}").get(), 0);
    // A control character between wide glyphs must not disturb their widths.
    assert_eq!(Cells::of("漢\u{7}字").get(), 4);
}

/// A name is data from the filesystem, not a terminal stream: what cannot be
/// drawn becomes one visible cell, so two entries differing only by an
/// invisible character cannot read as the same file.
#[test]
fn printable_name_replaces_the_undrawable_and_borrows_otherwise() {
    assert!(matches!(
        printable_name("ordinary.txt"),
        std::borrow::Cow::Borrowed("ordinary.txt")
    ));
    assert_eq!(printable_name("tab\there"), "tab\u{FFFD}here");
    assert_eq!(
        printable_name("bel\u{7}-cr\r"),
        format!("bel{UNPRINTABLE}-cr{UNPRINTABLE}")
    );
    // The replacement is one cell wide, so the row measures what it draws.
    assert_eq!(
        Cells::of(&printable_name("tab\there")).get(),
        "tab?here".len()
    );
}

/// A character that draws nothing is as good as a hidden one: two files whose
/// names differ only by a soft hyphen or a zero-width space would read as the
/// same row, and the bidi overrides can reverse an extension on screen.
#[test]
fn printable_name_shows_the_characters_that_draw_nothing() {
    let marker = UNPRINTABLE;
    for hidden in [
        '\u{00AD}', '\u{200B}', '\u{200E}', '\u{202E}', '\u{2060}', '\u{FEFF}',
    ] {
        let name = format!("cc{hidden}dd.txt");
        assert_eq!(
            printable_name(&name),
            format!("cc{marker}dd.txt"),
            "{hidden:?} passed through unseen"
        );
    }
    // A name that only looks similar is left exactly as it is.
    assert!(matches!(
        printable_name("ccdd.txt"),
        std::borrow::Cow::Borrowed("ccdd.txt")
    ));
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

/// Far shows Name | Size | Date | Time, and drops the trailing columns — Time
/// first — rather than let the name fall below [`MIN_NAME_WIDTH`].
///
/// Dropping from the left instead would leave a Time column beside rows with
/// no names in them, which is the failure this ordering exists to prevent.
#[test]
fn test_panel_layout_sheds_columns_from_the_right_on_a_narrow_panel() {
    // 78 interior cells: everything fits, name takes the rest.
    let wide = panel_layout(PanelView::Full, Cells::new(78));
    assert_eq!(wide.columns.len(), 3);
    assert_eq!(wide.name, Cells::new(78 - (6 + 1) - (8 + 1) - (5 + 1)));

    // 38 interior cells (an 80-column screen split in two, less borders) still
    // affords all four columns — this is the everyday case.
    assert_eq!(
        panel_layout(PanelView::Full, Cells::new(38)).columns.len(),
        3
    );

    // 28 cells cannot: Time goes, and the name clears its floor again.
    let narrow = panel_layout(PanelView::Full, Cells::new(28));
    assert_eq!(narrow.columns.len(), 2);
    assert!(narrow.name >= Cells::new(8));

    // 18 cells keep only Size, and 8 cells keep nothing but the name.
    assert_eq!(
        panel_layout(PanelView::Full, Cells::new(18)).columns.len(),
        1
    );
    assert_eq!(
        panel_layout(PanelView::Full, Cells::new(8)).columns.len(),
        0
    );
}

/// The Size column names what a row *is* before it measures it.
#[test]
fn test_size_field_speaks_fars_vocabulary() {
    let dir = DirEntry {
        name: "docs".to_string(),
        kind: EntryKind::Directory,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };
    assert_eq!(size_field(&dir), "Folder");

    let parent = DirEntry {
        name: "..".to_string(),
        ..dir
    };
    assert_eq!(size_field(&parent), "Up");

    // A byte count is printed outright while it fits the seven-cell column.
    let file = DirEntry {
        name: "a.txt".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(78_743),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };
    assert_eq!(size_field(&file), "78743");

    // Past that it is abbreviated rather than clipped to a wrong number.
    let big = DirEntry {
        size: FileSize::from_bytes(12_345_678),
        ..file
    };
    assert_eq!(size_field(&big), "11.8 M");
}

/// Far's compact byte count keeps a decimal only while there is room for one.
#[test]
fn test_abbreviate_bytes_matches_fars_footer_format() {
    assert_eq!(abbreviate_bytes(0), "0");
    assert_eq!(abbreviate_bytes(999), "999");
    assert_eq!(abbreviate_bytes(390_144), "381 K");
    assert_eq!(abbreviate_bytes(53_050_064_896), "49.4 G");
}

/// A file's name splits at its FINAL dot; a directory's never splits, and
/// neither does a leading dot — `.env` is a hidden file, not an empty name
/// with an "env" extension.
#[test]
fn test_split_extension_follows_fars_rules() {
    let file = |name: &str| DirEntry {
        name: name.to_string(),
        kind: EntryKind::File,
        ..DirEntry::default()
    };

    assert_eq!(
        split_extension(&file("Receipt-2893.pdf")),
        ("Receipt-2893", "pdf")
    );
    assert_eq!(
        split_extension(&file("docs.code-workspace")),
        ("docs", "code-workspace")
    );
    assert_eq!(
        split_extension(&file("archive.tar.gz")),
        ("archive.tar", "gz")
    );
    assert_eq!(split_extension(&file(".env")), (".env", ""));
    assert_eq!(split_extension(&file("README")), ("README", ""));

    let dir = DirEntry {
        name: ".cargo".to_string(),
        kind: EntryKind::Directory,
        ..DirEntry::default()
    };
    assert_eq!(split_extension(&dir), (".cargo", ""));
}

/// The extension is right-aligned against the Size column, so a listing lines
/// up by type without being sorted by it.
#[test]
fn test_name_field_right_aligns_the_extension() {
    let entry = DirEntry {
        name: "a.txt".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };
    assert_eq!(name_field(&entry, Cells::new(10)), "a      txt");

    // Down to one cell for the base the split still pays for itself: the dot
    // becomes the gap, which is exactly what the extension column is.
    assert_eq!(name_field(&entry, Cells::new(5)), "a txt");

    // Below that it would leave the base no room at all, so the undivided name
    // keeps it — a lone right-aligned "txt" identifies the file less than a
    // cut "a.t}" does.
    assert_eq!(name_field(&entry, Cells::new(4)), "a.t}");

    // A name longer than its column is cut with Far's marker, and the
    // extension still lands against the right edge.
    let long = DirEntry {
        name: "очень-длинное-имя.txt".to_string(),
        ..entry
    };
    let field = name_field(&long, Cells::new(12));
    assert_eq!(Cells::of(&field), Cells::new(12));
    assert!(field.ends_with("txt"), "got: '{field}'");
    assert!(field.contains('}'), "got: '{field}'");
}

/// Whether the row ends with a shell prompt and `text`.
///
/// `#` or `$` depending on the euid, so a suite run as either passes.
fn ends_with_prompt(row: &str, text: &str) -> bool {
    row.ends_with(&format!("# {text}")) || row.ends_with(&format!("$ {text}"))
}

/// With the caret parked mid-line, the ghost suggestion is not offered — the
/// accept gesture only makes sense at the end of the line — and the run after
/// the caret comes from the text that was typed there, not from the ghost.
#[test]
fn command_line_mid_caret_hides_the_ghost_and_shows_the_typed_tail() {
    use crate::models::CommandLine;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    let mut state = CommandLine::from("ls file.txt");
    // Split the line so the caret sits between "ls " and "file.txt" instead
    // of at the end, where `at_end()` would offer the ghost.
    state.line.set_split("ls ", "file.txt");
    state.ghost = Some("ignored suggestion".to_owned());
    app.set_overlay(Overlay::CommandLine(state));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd_row = row_text(&terminal, 22);
    assert!(
        cmd_row.contains("file.txt"),
        "the typed tail after the caret must still show: '{cmd_row}'"
    );
    assert!(
        !cmd_row.contains("ignored suggestion"),
        "the ghost must not show while the caret sits mid-line: '{cmd_row}'"
    );
}

/// `Part::At` gets its own colour, distinct from the user and host segments
/// it sits between — the one part of the prompt whose whole job is to be
/// legible as a separator rather than as either name.
#[test]
fn prompt_spans_colours_the_at_between_user_and_host() {
    use crate::prompt::{Part, Segment};

    let prompt = vec![
        Segment {
            text: "root".to_owned(),
            part: Part::User,
        },
        Segment {
            text: "@".to_owned(),
            part: Part::At,
        },
        Segment {
            text: "box".to_owned(),
            part: Part::Host,
        },
    ];

    let spans = prompt_spans(&prompt);
    let at = spans
        .iter()
        .find(|span| span.content.as_ref() == "@")
        .expect("the '@' segment is drawn");
    assert_eq!(at.style.fg, Some(palette::PROMPT_AT));
}

/// A 20-column bar gives the last four keys a one-column cell. The key number
/// must be clipped to its cell like the label is, or every cell after "10"
/// drifts one column right of where `fkey_at` — and so a click — expects it.
#[test]
fn test_narrow_key_bar_keeps_every_cell_where_the_hit_test_expects_it() {
    const WIDTH: u16 = 20;
    let backend = TestBackend::new(WIDTH, 8);
    let mut terminal = Terminal::new(backend).unwrap();
    let (mut app, _temp) = blank_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let buf = terminal.backend().buffer();
    let y = buf.area.height - 1;
    let mut left = 0u16;
    let widths = crate::mouse::fkey_widths(WIDTH, FKEY_LABELS.len());
    for (index, width) in widths.into_iter().enumerate() {
        let (key, _) = FKEY_LABELS[index];
        assert_eq!(
            buf[(left, y)].symbol(),
            &key[..1],
            "key {} must start at column {left}, row: {:?}",
            index + 1,
            row_text(&terminal, y)
        );
        assert_eq!(
            crate::mouse::fkey_at(WIDTH, FKEY_LABELS.len(), left),
            u8::try_from(index + 1).ok(),
            "and the hit test must agree"
        );
        left += width;
    }
}
