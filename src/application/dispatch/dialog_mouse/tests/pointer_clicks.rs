//! Purpose: full clicks through `dispatch_dialog_mouse` — a menu's command row, a dialog's button and its body row — not just the pieces.

use super::*;

/// A click on an open menu's command row is the full handoff: it runs
/// `click_in_menu`, and if that leaves a key pending, the same call presses
/// it — exercising the branch `dispatch_dialog_mouse` takes when the overlay
/// is a [`Overlay::Menu`], not one of the plain dialogs the tests above drive
/// through `click_dialog_row` or `dialog_button_key` directly.
#[test]
fn clicking_a_menu_command_row_runs_its_key_and_closes_the_menu() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut term = terminal();
    let menu = Menu::open(Heading::Commands);
    app.set_menu(menu);
    // Tall enough for the drop-down to show "Go to root", the row the click
    // lands on. The Commands menu has 33 entries on Linux, separators
    // included, and the drop-down is clamped to the screen, so the rows that
    // do not fit are never drawn and cannot be clicked.
    let screen = ratatui::layout::Rect::new(0, 0, 80, 31);
    app.dialog_mouse.screen = screen;

    let (column, row) = menu_command_position(&menu, screen, "Go to root");

    dispatch_dialog_mouse(
        &mut term,
        &mut app,
        mouse(MouseEventKind::Down(MouseButton::Left), column, row),
    );

    assert!(
        matches!(app.overlay, Overlay::None),
        "the menu closed before the key ran"
    );
    assert_eq!(app.pending_row, None, "the chosen row was consumed");
    let root = std::path::PathBuf::from(if cfg!(windows) { "C:\\" } else { "/" });
    assert_eq!(
        app.get_active_panel().path,
        root,
        "the click ran Ctrl+\\ through the same dispatch a keypress uses"
    );
}

/// Clicking Commands → Find file must open the find-file dialog, never F7's
/// Make folder prompt. The row's shortcut is Alt+F7; a click that dropped the
/// Alt would create a directory instead of searching.
#[test]
fn clicking_find_file_opens_find_file_not_make_folder() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut term = terminal();
    let menu = Menu::open(Heading::Commands);
    app.set_menu(menu);
    app.dialog_mouse.screen = ratatui::layout::Rect::new(0, 0, 80, 24);
    click_text(&mut term, &mut app, "Find file");
    draw(&mut app, &mut term);
    let screen = crate::test_support::buffer_text(&term);
    assert!(
        !matches!(app.overlay, Overlay::NamePrompt(_)),
        "Find file must not open Make folder"
    );
    assert!(
        !screen.contains("Create the folder"),
        "must not show the F7 dialog:\n{screen}"
    );
    assert!(
        matches!(app.overlay, Overlay::FindFile(_)),
        "Commands → Find file must open the Find file overlay"
    );
    assert!(screen.contains("Find file"), "{screen}");
}

/// A click on a dialog's button, routed the same way the pointer really
/// delivers it: through `dialog_mouse.hit`, not straight into
/// `dialog_button_key` the way the tests above call it.
#[test]
fn clicking_a_dialog_button_through_the_pointer_presses_it() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut term = terminal();
    app.set_overlay(Overlay::QuitConfirm(Confirm::default()));
    draw(&mut app, &mut term);

    let &(index, left, _right) = app
        .dialog_mouse
        .buttons
        .first()
        .expect("the dialog drew its buttons");
    assert_eq!(index, 0, "the first button is Yes");
    let buttons_y = app.dialog_mouse.buttons_y;

    dispatch_dialog_mouse(
        &mut term,
        &mut app,
        mouse(MouseEventKind::Down(MouseButton::Left), left, buttons_y),
    );

    assert_eq!(
        app.lifecycle,
        crate::models::Lifecycle::Quitting,
        "the click on Yes pressed the key it stands for"
    );
}

/// A click on a dialog's body row, routed through `dialog_mouse.hit` rather
/// than straight into `click_dialog_row` the way the earlier tests call it.
#[test]
fn clicking_a_dialog_body_row_through_the_pointer_reaches_the_field() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut term = terminal();
    app.set_overlay(Overlay::CopyPrompt(Box::new(CopyPrompt::new(
        "a.txt",
        Path::new("/tmp"),
    ))));
    draw(&mut app, &mut term);

    let body = app.dialog_mouse.body.expect("the body was drawn");
    dispatch_dialog_mouse(
        &mut term,
        &mut app,
        mouse(MouseEventKind::Down(MouseButton::Left), body.x, body.y + 6),
    );

    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("copy dialog");
    };
    assert_eq!(prompt.field, CopyField::Timestamps);
    assert!(prompt.timestamps, "the checkbox answered the click");
}

fn menu_command_position(menu: &Menu, screen: ratatui::layout::Rect, caption: &str) -> (u16, u16) {
    let index = menu
        .entries()
        .iter()
        .position(|entry| {
            matches!(entry, crate::menu::Entry::Item { label, .. } if label.contains(caption))
        })
        .expect("the root command is on the Commands menu");
    let (column, row) = (0..screen.height)
        .flat_map(|row| (0..screen.width).map(move |column| (column, row)))
        .find(|&(column, row)| crate::ui::menu_row_at(menu, screen, column, row) == Some(index))
        .expect("some cell must land on the row that was just found");
    (column, row)
}
