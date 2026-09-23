//! Unit tests: the Video mode status line, for a terminal that resized and
//! for one that ignored the request.

use super::*;

#[test]
fn a_terminal_that_resized_reports_its_new_height() {
    let outcome = VideoMode {
        requested_rows: 50,
        rows: 50,
    };
    assert_eq!(describe(outcome), "Video mode: 50 rows");
}

#[test]
fn a_terminal_that_kept_its_height_is_named_as_the_reason() {
    let outcome = VideoMode {
        requested_rows: 25,
        rows: 40,
    };
    assert_eq!(
        describe(outcome),
        "Video mode: asked for 25 rows, the terminal has 40"
    );
}

/// The resize itself is skipped here so the test does not change the host
/// terminal; the skipped call stands for a terminal that ignored it.
#[test]
fn toggle_reports_in_the_status_bar() {
    let _guard = crate::terminal::skip_video_mode_resize();
    let mut app = App::with_panels(
        crate::models::Panel::from_entries(std::path::PathBuf::from("/srv/x"), Vec::new()),
        crate::models::Panel::from_entries(std::path::PathBuf::from("/srv/x"), Vec::new()),
    );
    toggle(&mut app).expect("toggle");
    assert!(
        app.status
            .to_string()
            .starts_with("Video mode: asked for 25 rows"),
        "{}",
        app.status
    );
}
