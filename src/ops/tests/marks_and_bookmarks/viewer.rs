//! Purpose: The viewer on a one-line file has nothing to scroll.

use super::*;

// --- Viewer with single line ---

#[test]
fn test_process_popup_input_viewer_single_line_no_scroll() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "one.txt",
        vec!["only line".to_owned()],
    )));

    // Try to scroll down - should stay at 0
    assert!(process_popup_input(&mut app, key_event(KeyCode::Down)));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    Ok(())
}
