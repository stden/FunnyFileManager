//! Purpose: the command-line row, the menu bar, the F-key bar and the event loop's mouse path.

use super::*;

/// A click on the command-line row opens it, and a second one does not throw
/// away what has been typed into it.
#[test]
fn clicking_the_command_line_row_opens_it_once() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt"]);
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let mut clicks = crate::mouse::PanelClicks::default();
    let command_row = 22;

    click_at(&mut terminal, &mut app, (10, command_row), &mut clicks);
    let Overlay::CommandLine(_) = &app.overlay else {
        panic!("the command line opened");
    };

    if let Overlay::CommandLine(line) = &mut app.overlay {
        line.line.insert_str("echo hi");
    }
    click_at(&mut terminal, &mut app, (10, command_row), &mut clicks);
    let Overlay::CommandLine(line) = &app.overlay else {
        panic!("still open");
    };
    assert_eq!(
        line.line.text(),
        "echo hi",
        "a second click must not clear it"
    );
}

/// A click on one of the bar's headings opens its drop-down, the same as
/// pressing its hotkey would.
#[test]
fn clicking_a_heading_on_the_bar_opens_its_menu() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt"]);
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    let bar = ratatui::layout::Rect { height: 1, ..area };
    let column = (0..area.width)
        .find(|&c| crate::ui::heading_at(bar, c) == Some(menu::Heading::Files))
        .expect("the Files heading is drawn on the bar");

    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse_at(MouseEventKind::Down(MouseButton::Left), column, area.y),
        area,
        &mut crate::mouse::PanelClicks::default(),
    );

    let Overlay::Menu(menu) = &app.overlay else {
        panic!("clicking a heading opens its drop-down");
    };
    assert_eq!(menu.heading, menu::Heading::Files);
}

/// The bottom bar's cells are the F-keys; clicking one runs it exactly as the
/// key would, through the same dispatcher.
#[test]
fn clicking_an_f_key_cell_runs_that_f_key() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt"]);
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    assert!(!app.overlay.is_active());

    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse_at(MouseEventKind::Down(MouseButton::Left), 0, area.height - 1),
        area,
        &mut crate::mouse::PanelClicks::default(),
    );

    assert!(
        matches!(app.overlay, Overlay::Help),
        "the first cell of the bottom bar is F1, Help"
    );
}

/// The event loop itself — not just `dispatch_mouse` in isolation — routes a
/// mouse event to the panels and F-keys the same way it routes a keypress.
#[test]
fn the_event_loop_routes_a_mouse_click_through_dispatch_mouse() {
    let (app, mut terminal, _dir) = app_and_terminal(&["a.txt"]);

    let mut events = vec![Event::Mouse(mouse_at(
        MouseEventKind::Down(MouseButton::Left),
        0,
        23,
    ))]
    .into_iter();

    let err = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "stop after the click"))
    })
    .expect_err("the loop reads on after handling the click");

    assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    let screen = buffer_text(&terminal);
    assert!(
        screen.contains("Far Manager style navigation:"),
        "a click on the F1 cell reached the help overlay through the mouse path: {screen}"
    );
}
