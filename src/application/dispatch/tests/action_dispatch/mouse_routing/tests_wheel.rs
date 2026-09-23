//! Purpose: the mouse wheel over a panel, the console transcript, the bar and empty screen.

use super::*;

/// The wheel moves the cursor, which is the one mouse gesture people use
/// without thinking about it.
#[test]
fn the_wheel_moves_the_panel_cursor_both_ways() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt", "b.txt", "c.txt", "d.txt"]);
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    app.left_panel.select(Some(0));

    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse_at(MouseEventKind::ScrollDown, 10, 5),
        area,
        &mut crate::mouse::PanelClicks::default(),
    );
    let after_down = app.left_panel.selected();
    assert_eq!(after_down, Some(1), "the wheel moved the cursor down");

    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse_at(MouseEventKind::ScrollUp, 10, 5),
        area,
        &mut crate::mouse::PanelClicks::default(),
    );
    assert_eq!(app.left_panel.selected(), Some(0));
}

/// With the transcript full-screen, the wheel scrolls that transcript no
/// matter which screen cell the pointer happens to be over.
#[test]
fn the_wheel_scrolls_the_console_and_returns_to_the_same_line() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt"]);
    for line in 0..100 {
        app.console.record_output(&format!("line {line}"));
    }
    app.view = app.view.toggled();
    terminal.draw(|frame| ui(frame, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    press(&mut app, &mut terminal, KeyCode::Home);
    let at_top = newest(&app);
    let mut clicks = crate::mouse::PanelClicks::default();

    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse_at(MouseEventKind::ScrollDown, area.x, area.y),
        area,
        &mut clicks,
    );
    assert_ne!(newest(&app), at_top, "the transcript moved down one line");

    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse_at(MouseEventKind::ScrollUp, area.x, area.y),
        area,
        &mut clicks,
    );
    assert_eq!(newest(&app), at_top, "scrolling back restored the view");
}

/// In panel mode the wheel belongs to the panel under the pointer. Over the
/// menu bar there is no panel, so the current listing must stay put.
#[test]
fn the_wheel_over_the_menu_bar_does_not_move_a_panel() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt", "b.txt", "c.txt"]);
    terminal.draw(|frame| ui(frame, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    app.left_panel.select(Some(1));
    let before = app.left_panel.selected();

    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse_at(MouseEventKind::ScrollDown, area.x, area.y),
        area,
        &mut crate::mouse::PanelClicks::default(),
    );

    assert_eq!(app.left_panel.selected(), before);
}

/// A wheel event over nothing the panels or the bar answer to must not move a
/// panel that happens to be scrolled to behind the pointer.
#[test]
fn scrolling_over_nothing_does_not_move_any_panel() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt", "b.txt"]);
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    let before = app.left_panel.selected();

    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse_at(MouseEventKind::ScrollDown, area.width + 5, 5),
        area,
        &mut crate::mouse::PanelClicks::default(),
    );

    assert_eq!(
        app.left_panel.selected(),
        before,
        "a scroll past the edge of the screen hits nothing"
    );
}

/// While the console is showing, the wheel scrolls the transcript — the same
/// place the arrow keys go — rather than a panel nobody can see.
#[test]
fn the_wheel_scrolls_the_console_transcript_when_it_is_showing() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt"]);
    for line in 0..50 {
        app.console.record_output(&format!("line {line}"));
    }
    app.view = app.view.toggled();
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let area = terminal.get_frame().area();

    let newest = |app: &App| {
        app.console
            .visible(10)
            .last()
            .map(|line| line.text().to_owned())
            .unwrap_or_default()
    };
    let before = newest(&app);

    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse_at(MouseEventKind::ScrollUp, 10, 5),
        area,
        &mut crate::mouse::PanelClicks::default(),
    );

    assert_ne!(
        newest(&app),
        before,
        "the wheel scrolled the transcript, not a hidden panel"
    );
}
