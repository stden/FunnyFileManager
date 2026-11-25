//! Purpose: unit tests for the F3 view scrolling within what the frame drew.

use super::*;

#[test]
fn the_view_scrolls_within_what_the_frame_drew() {
    let mut editor = editor(&json!({}));
    editor.set_mode(Mode::View(ScrollTop::TOP));
    editor.record_drawn(Drawn::View {
        lines: 50,
        page: 20,
    });
    editor.scroll_view(Step::Last);
    assert_eq!(editor.mode(), &Mode::View(ScrollTop::TOP.down(30, 30)));
    editor.scroll_view(Step::Down);
    assert_eq!(
        editor.mode(),
        &Mode::View(ScrollTop::TOP.down(30, 30)),
        "stops at the end"
    );
    editor.scroll_view(Step::PageUp);
    assert_eq!(editor.mode(), &Mode::View(ScrollTop::TOP.down(10, 30)));
    editor.scroll_view(Step::Up);
    editor.scroll_view(Step::PageDown);
    assert_eq!(editor.mode(), &Mode::View(ScrollTop::TOP.down(29, 30)));
    editor.scroll_view(Step::First);
    assert_eq!(editor.mode(), &Mode::View(ScrollTop::TOP));
}

#[test]
fn a_frame_with_fewer_lines_pulls_the_view_back_in_range() {
    let mut editor = editor(&json!({}));
    editor.set_mode(Mode::View(ScrollTop::TOP));
    editor.record_drawn(Drawn::View {
        lines: 50,
        page: 20,
    });
    editor.scroll_view(Step::Last);
    editor.record_drawn(Drawn::View {
        lines: 25,
        page: 20,
    });
    assert_eq!(editor.mode(), &Mode::View(ScrollTop::TOP.down(5, 5)));
}
