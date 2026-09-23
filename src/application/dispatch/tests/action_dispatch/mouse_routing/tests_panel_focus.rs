//! Purpose: a click makes the panel under the pointer active, from any view or overlay.

use super::*;

/// A press on the other panel's body makes that panel the active one, which is
/// how a mouse user switches sides.
#[test]
fn clicking_the_other_panel_makes_it_active() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt", "b.txt"]);
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    assert_eq!(app.active_panel, PanelSide::Left);

    let mut clicks = crate::mouse::PanelClicks::default();
    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse_at(MouseEventKind::Down(MouseButton::Left), 60, 5),
        area,
        &mut clicks,
    );

    assert_eq!(app.active_panel, PanelSide::Right);
}

/// The command line is part of the main layout rather than a floating dialog.
/// Clicking a panel therefore dismisses the unfinished command and gives that
/// panel the same focus and row selection it would receive while idle.
#[test]
fn clicking_a_panel_from_the_command_line_returns_focus_to_it() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt", "b.txt"]);
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    let column = area.x + (area.width * 3 / 4);
    let row = area.y + 4;
    assert!(matches!(
        crate::mouse::hit(
            area,
            column,
            row,
            crate::ui::FKEY_COUNT,
            models::PanelsShown::Both
        ),
        crate::mouse::Target::PanelRow {
            side: PanelSide::Right,
            ..
        }
    ));
    app.set_overlay(Overlay::CommandLine("unfinished command".into()));

    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse_at(MouseEventKind::Down(MouseButton::Left), column, row),
        area,
        &mut crate::mouse::PanelClicks::default(),
    );

    assert_eq!(app.active_panel, PanelSide::Right);
    assert!(!app.overlay.is_active(), "the panel now owns input focus");
    assert_eq!(app.right_panel.selected(), Some(1), "the row was selected");
}

/// A command line can also sit over the full-screen console. There is no panel
/// under the pointer in that view, so its geometry must not steal focus.
#[test]
fn clicking_the_console_does_not_pretend_a_hidden_panel_was_clicked() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt"]);
    app.view = ViewMode::Console;
    app.set_overlay(Overlay::CommandLine("echo still editing".into()));
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let area = terminal.get_frame().area();

    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse_at(MouseEventKind::Down(MouseButton::Left), 60, 5),
        area,
        &mut crate::mouse::PanelClicks::default(),
    );

    assert_eq!(app.get_command_line(), Some("echo still editing"));
    assert_eq!(app.active_panel, PanelSide::Left);
}

/// Panel chrome belongs to its panel even though it is not an entry row. A
/// click on the other panel's column heading switches sides without moving its
/// cursor.
#[test]
fn clicking_the_other_panels_header_activates_it_without_selecting_a_row() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt", "b.txt"]);
    terminal.draw(|frame| ui(frame, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    let column = area.x + (area.width * 3 / 4);
    let row = area.y + 2;
    assert_eq!(
        crate::mouse::hit(
            area,
            column,
            row,
            crate::ui::FKEY_COUNT,
            models::PanelsShown::Both
        ),
        crate::mouse::Target::Panel(PanelSide::Right),
        "test setup: the pointer is on the right column heading"
    );
    let before = app.right_panel.selected();

    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse_at(MouseEventKind::Down(MouseButton::Left), column, row),
        area,
        &mut crate::mouse::PanelClicks::default(),
    );

    assert_eq!(app.active_panel, PanelSide::Right);
    assert_eq!(app.right_panel.selected(), before);
}

/// A click on a panel's frame — its header row, above the first entry — still
/// makes that side active, the way clicking anywhere on it would, but it does
/// not pick a row: there is no row under a header.
#[test]
fn clicking_a_panels_header_row_makes_it_active_without_selecting_a_row() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt", "b.txt"]);
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    let before = app.right_panel.selected();
    assert_eq!(app.active_panel, PanelSide::Left);

    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse_at(MouseEventKind::Down(MouseButton::Left), 60, 1),
        area,
        &mut crate::mouse::PanelClicks::default(),
    );

    assert_eq!(
        app.active_panel,
        PanelSide::Right,
        "the frame still switches panels"
    );
    assert_eq!(
        app.right_panel.selected(),
        before,
        "but the header row picked no entry"
    );
}
