//! Unit tests: the quick view's preview, its scroll and its page geometry.

use std::path::PathBuf;

use super::{Preview, QuickView, Refusal, TextPreview};
use crate::encoding::TextEncoding;
use crate::models::Viewport;
use crate::scroll::Scrolled;

fn text(lines: &[&str]) -> Preview {
    Preview::Text(TextPreview::decoded(
        &PathBuf::from("notes.txt"),
        lines.join("\n").as_bytes(),
    ))
}

#[test]
fn decoding_follows_the_viewer_rules() {
    let utf8 = TextPreview::decoded(&PathBuf::from("a.txt"), b"one\ntwo\n");
    assert_eq!(utf8.lines, vec!["one", "two"]);
    assert_eq!(utf8.encoding, TextEncoding::Utf8);
    assert_eq!(utf8.size.bytes(), 8);

    let single_byte = TextPreview::decoded(&PathBuf::from("a.txt"), &[0xE9, b'\n']);
    assert_eq!(single_byte.lines, vec!["\u{439}"]);
    assert_eq!(single_byte.encoding, TextEncoding::Windows1251);

    let empty = TextPreview::decoded(&PathBuf::from("a.txt"), b"");
    assert_eq!(empty.lines, vec!["<empty file>"]);
}

#[test]
fn showing_a_new_file_starts_at_the_top_left() {
    let mut view = QuickView::default();
    view.show(Some(PathBuf::from("a")), text(&["1", "2", "3", "4", "5"]));
    view.scroll_down(3, 2);
    view.scroll_right(4);
    assert_eq!(view.scroll.lines(), 3);
    assert_eq!(view.col.cells(), 4);

    view.show(Some(PathBuf::from("b")), text(&["x"]));
    assert_eq!(view.shown, Some(PathBuf::from("b")));
    assert_eq!(view.scroll.lines(), 0);
    assert_eq!(view.col.cells(), 0);
}

#[test]
fn the_last_line_stops_on_the_bottom_row() {
    let mut view = QuickView::default();
    view.show(None, text(&["1", "2", "3", "4", "5"]));
    view.scroll_down(10, 2);
    assert_eq!(view.scroll.lines(), 3);
    view.scroll_to_bottom(4);
    assert_eq!(view.scroll.lines(), 1);
    view.scroll_to_top();
    assert_eq!(view.scroll.lines(), 0);
}

#[test]
fn a_summary_or_a_refusal_has_nothing_to_scroll() {
    let mut view = QuickView::default();
    view.show(None, Preview::Refused(Refusal::Binary));
    view.scroll_down(5, 3);
    assert_eq!(view.scroll.lines(), 0);
    assert_eq!(view.line_count(), 0);
}

#[test]
fn the_page_is_the_body_of_the_panel_frame() {
    // 24 rows: three around the panels, two of frame, the rule and the info
    // row.
    assert_eq!(QuickView::page_for(Viewport::new(24)), 17);
    assert_eq!(QuickView::page_for(Viewport::new(3)), 1);
}

#[test]
fn a_refusal_explains_itself() {
    assert_eq!(Refusal::Binary.label(), "Binary file: no text to show");
    assert_eq!(Refusal::NotRegular.label(), "Not a regular file");
    assert_eq!(
        Refusal::Unreadable("denied".to_owned()).label(),
        "Cannot read: denied"
    );
    assert!(
        Refusal::TooLarge(crate::models::FileSize::from_bytes(12 << 20))
            .label()
            .starts_with("Too large to preview: 12.0 MB")
    );
}
