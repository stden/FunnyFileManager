//! Unit tests: the quick-view panel draws the preview it carries — text with
//! the viewer's colours, a folder's totals, or the reason there is nothing.

use std::path::PathBuf;

use ratatui::Terminal;

use super::render;
use crate::encoding::TextEncoding;
use crate::models::{App, FileSize, Panel, PanelKind, Preview, QuickView, Refusal, TextPreview};
use crate::scroll::Scrolled;
use crate::stats::{Coverage, DirectoryStats};
use crate::test_support::{buffer_text, WritableTestBackend};

fn draw(view: QuickView, is_active: bool) -> String {
    let mut terminal = Terminal::new(WritableTestBackend::new(40, 12)).expect("terminal");
    let mut app = App::new().expect("app");
    app.right_panel.kind = PanelKind::QuickView(Box::new(view));
    terminal
        .draw(|f| render(f, &app.right_panel, f.area(), is_active))
        .expect("draw");
    buffer_text(&terminal)
}

fn text_view(path: &str, lines: &[&str]) -> QuickView {
    let mut view = QuickView::default();
    view.show(
        Some(PathBuf::from(path)),
        Preview::Text(TextPreview::decoded(
            &PathBuf::from(path),
            lines.join("\n").as_bytes(),
        )),
    );
    view
}

#[test]
fn a_text_preview_shows_the_path_the_lines_and_the_info_row() {
    let screen = draw(
        text_view("/tmp/notes.txt", &["first line", "second"]),
        false,
    );
    assert!(screen.contains("/tmp/notes.txt"), "{screen}");
    assert!(screen.contains("first line"), "{screen}");
    assert!(screen.contains("second"), "{screen}");
    assert!(screen.contains("UTF-8"), "{screen}");
    assert!(screen.contains("17 bytes"), "{screen}");
    assert!(screen.contains("100%"), "{screen}");
}

#[test]
fn a_folder_description_is_highlighted_like_its_file() {
    let mut view = text_view("/srv/project.rs/README.md", &["# Folder notes"]);
    let Preview::Text(text) = &view.preview else {
        panic!("expected text");
    };
    let body = ratatui::layout::Rect::new(0, 0, 40, 8);
    let expected = super::text_lines(&view, text, body);
    view.shown = Some(PathBuf::from("/srv/project.rs"));
    assert_eq!(super::text_lines(&view, text, body), expected);
}

#[test]
fn the_body_scrolls_with_the_view() {
    let lines: Vec<String> = (1..=30).map(|n| format!("row {n}")).collect();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let mut view = text_view("/tmp/long.txt", &refs);
    view.scroll_down(5, 6);
    view.scroll_right(2);
    let screen = draw(view, true);
    assert!(!screen.contains("row 1 "), "{screen}");
    assert!(screen.contains("w 6"), "scrolled two cells right: {screen}");
    assert!(!screen.contains("row 6"), "{screen}");
    assert!(!screen.contains("100%"), "{screen}");
}

#[test]
fn the_body_has_exactly_the_rows_the_page_says() {
    // 12 rows: the frame's two, the rule and the info row leave eight for
    // text — the number `QuickView::page_for` gives once the three chrome
    // rows around the panels are added back.
    let lines: Vec<String> = (1..=30).map(|n| format!("row {n}")).collect();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let screen = draw(text_view("/tmp/long.txt", &refs), false);
    assert!(screen.contains("row 8"), "{screen}");
    assert!(!screen.contains("row 9"), "{screen}");
    assert_eq!(QuickView::page_for(crate::models::Viewport::new(15)), 8);
}

#[test]
fn a_folder_preview_lists_its_totals() {
    let mut view = QuickView::default();
    view.show(
        Some(PathBuf::from("/srv/project")),
        Preview::Folder(Box::new(DirectoryStats {
            total: FileSize::from_bytes(3 * 1024 * 1024),
            files: 42,
            directories: 7,
            coverage: Coverage::Partial,
            ..DirectoryStats::default()
        })),
    );
    let screen = draw(view, false);
    assert!(screen.contains("/srv/project"), "{screen}");
    assert!(screen.contains("Folder"), "{screen}");
    assert!(screen.contains("Total size"), "{screen}");
    assert!(screen.contains("3.0 M"), "{screen}");
    assert!(screen.contains("Files"), "{screen}");
    assert!(screen.contains("42"), "{screen}");
    assert!(screen.contains("Folders"), "{screen}");
    assert!(screen.contains('7'), "{screen}");
    assert!(screen.contains("lower bounds"), "{screen}");
}

/// A folder that could not be read says so, in the words an unreadable file
/// gets, instead of "Files: 0" — and its footer claims no counts either.
#[test]
fn an_unreadable_folder_says_it_cannot_be_read() {
    let mut view = QuickView::default();
    view.show(
        Some(PathBuf::from("/srv/x/no-access")),
        Preview::Folder(Box::new(DirectoryStats {
            coverage: Coverage::Unreadable("Permission denied (os error 13)".to_owned()),
            ..DirectoryStats::default()
        })),
    );
    let screen = draw(view, false);
    assert!(screen.contains("Cannot read: Permission"), "{screen}");
    assert!(!screen.contains("Files"), "{screen}");
    assert!(!screen.contains("0 files"), "{screen}");
}

#[test]
fn a_folder_with_unread_folders_inside_marks_its_totals_as_lower_bounds() {
    let mut view = QuickView::default();
    view.show(
        Some(PathBuf::from("/srv/x/perms")),
        Preview::Folder(Box::new(DirectoryStats {
            files: 2,
            coverage: Coverage::Unread(1),
            ..DirectoryStats::default()
        })),
    );
    let screen = draw(view, false);
    assert!(screen.contains("unreadable folder"), "{screen}");
    assert!(screen.contains("lower bounds"), "{screen}");
}

#[test]
fn a_refusal_says_why_and_an_empty_view_says_nothing() {
    let mut view = QuickView::default();
    view.show(
        Some(PathBuf::from("/tmp/blob.bin")),
        Preview::Refused(Refusal::Binary),
    );
    let screen = draw(view, false);
    assert!(screen.contains("Binary file"), "{screen}");

    let screen = draw(QuickView::default(), true);
    assert!(screen.contains("Quick view"), "{screen}");
    assert!(!screen.contains("bytes"), "{screen}");
}

#[test]
fn a_single_byte_file_is_labelled_like_the_viewer_does() {
    let mut view = QuickView::default();
    let text = TextPreview::decoded(&PathBuf::from("/tmp/cp.txt"), &[0xE9, b'\n']);
    assert_eq!(text.encoding, TextEncoding::Windows1251);
    view.show(Some(PathBuf::from("/tmp/cp.txt")), Preview::Text(text));
    let screen = draw(view, false);
    assert!(screen.contains("Windows-1251"), "{screen}");
}

#[test]
fn a_file_panel_handed_here_draws_nothing_and_does_not_panic() {
    let mut terminal = Terminal::new(WritableTestBackend::new(40, 12)).expect("terminal");
    let dir = tempfile::TempDir::new().expect("temp dir");
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    terminal
        .draw(|f| render(f, &panel, f.area(), false))
        .expect("draw");
    let screen = buffer_text(&terminal);
    assert!(!screen.contains("Quick view"), "{screen}");
}
