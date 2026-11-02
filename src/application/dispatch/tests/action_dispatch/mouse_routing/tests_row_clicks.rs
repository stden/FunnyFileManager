//! Purpose: row clicks, double-clicks and the presses that must be swallowed.

use super::*;

/// A blank row is still inside the listing frame, but it names no entry. It
/// must break a pending double-click so the next real click only selects the
/// directory instead of unexpectedly entering it.
#[test]
fn clicking_a_blank_row_breaks_the_pending_double_click() {
    let (mut app, mut terminal, dir) = app_and_terminal(&[]);
    fs::create_dir(dir.path().join("project")).expect("test setup");
    app.left_panel.load_entries().expect("reload");
    terminal.draw(|frame| ui(frame, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    let project = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == "project")
        .expect("project is listed");
    let offset = app.left_panel.list_state_mut().offset();
    let visible_project = project.checked_sub(offset).expect("project is visible");
    let entries = app.left_panel.entries.len();
    let column = area.x + 10;
    let project_row = panel_row_at(area, column, |row| row == visible_project);
    let blank_row = panel_row_at(area, column, |row| offset + row >= entries);
    let original_path = app.left_panel.path.clone();
    let mut clicks = crate::mouse::PanelClicks::default();

    for row in [project_row, blank_row, project_row] {
        click_at(&mut terminal, &mut app, (column, row), &mut clicks);
    }

    assert_eq!(
        app.left_panel.path, original_path,
        "the click after a blank row selects; it does not complete a double-click"
    );
    assert_eq!(app.left_panel.selected(), Some(project));
}

/// A release, and a press on nothing, both have to be swallowed rather than
/// acted on — otherwise every drag ends in a second command.
#[test]
fn a_release_and_a_press_on_nothing_do_nothing() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt"]);
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    let before = app.left_panel.selected();
    let mut clicks = crate::mouse::PanelClicks::default();

    for kind in [MouseEventKind::Up(MouseButton::Left), MouseEventKind::Moved] {
        dispatch_mouse(
            &mut terminal,
            &mut app,
            mouse_at(kind, 10, 5),
            area,
            &mut clicks,
        );
    }
    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse_at(MouseEventKind::Down(MouseButton::Left), 0, 0),
        area,
        &mut clicks,
    );

    assert_eq!(app.left_panel.selected(), before);
    assert!(!app.overlay.is_active());
}

/// A second press on the same row, before the double-click window closes, is
/// Enter — opening the directory under the cursor exactly as the key would.
#[test]
fn a_double_click_on_a_directory_row_opens_it() {
    let (mut app, mut terminal, dir) = app_and_terminal(&[]);
    fs::create_dir(dir.path().join("subdir")).expect("test setup");
    app.left_panel.load_entries().expect("reload");
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    let before = app.left_panel.path.clone();
    let mut clicks = crate::mouse::PanelClicks::default();

    // Row 4 of an 80x24 screen is the left panel's second listed row: ".." is
    // first, "subdir" is the only other entry.
    for _ in 0..2 {
        dispatch_mouse(
            &mut terminal,
            &mut app,
            mouse_at(MouseEventKind::Down(MouseButton::Left), 10, 4),
            area,
            &mut clicks,
        );
    }

    assert_ne!(
        app.left_panel.path, before,
        "the second click activated the row"
    );
    assert!(app.left_panel.path.ends_with("subdir"));
}

/// A click past the last real row must not select anything or arm a
/// double-click — there is no entry there to select or to open.
#[test]
fn clicking_past_the_last_row_does_not_arm_a_double_click() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&[]);
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    let before = app.left_panel.selected();
    let mut clicks = crate::mouse::PanelClicks::default();

    // Only ".." is listed, so the row at index 1 (screen row 4) is past it.
    for _ in 0..2 {
        dispatch_mouse(
            &mut terminal,
            &mut app,
            mouse_at(MouseEventKind::Down(MouseButton::Left), 10, 4),
            area,
            &mut clicks,
        );
    }

    assert_eq!(app.left_panel.selected(), before, "nothing there to select");
    assert!(!app.overlay.is_active(), "and nothing there to open");
}
