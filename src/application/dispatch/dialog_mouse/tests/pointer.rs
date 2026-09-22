//! Purpose: the pointer routed through `dispatch_dialog_mouse`: wheel, drag,
//! presses outside, and full clicks on menus and dialogs.

use super::*;

// ─── the pointer inside a dialog ────────────────────────────────────────────

/// The wheel is the one gesture people use without thinking, and inside a
/// dialog it has to reach the same keys the arrows do.
#[test]
fn the_wheel_inside_a_dialog_moves_the_cursor() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut term = terminal();
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    draw(&mut app, &mut term);

    dispatch_dialog_mouse(
        &mut term,
        &mut app,
        mouse(MouseEventKind::ScrollDown, 40, 12),
    );
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    let moved = prompt.field();
    assert_ne!(
        moved,
        crate::models::NameField::Name,
        "the wheel moved down"
    );

    dispatch_dialog_mouse(&mut term, &mut app, mouse(MouseEventKind::ScrollUp, 40, 12));
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert_eq!(prompt.field(), crate::models::NameField::Name);
}

/// Far lets a dialog be pushed aside by any blank part of it, which is how you
/// look at what is underneath. The grip has to survive the drag and be let go
/// on release, or the dialog follows the pointer forever.
#[test]
fn a_dialog_is_dragged_by_its_chrome_and_let_go_on_release() {
    use MouseButton::Left;
    use MouseEventKind::{Down, Drag, Up};
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut term = terminal();
    app.set_overlay(Overlay::QuitConfirm(Confirm::Yes));
    draw(&mut app, &mut term);

    let area = app.dialog_mouse.area.expect("the dialog was drawn");
    let (grip_x, grip_y) = (area.x, area.y);
    gesture(&mut term, &mut app, Down(Left), (grip_x, grip_y));
    assert_eq!(app.dialog_mouse.grabbed, Some((grip_x, grip_y)));

    gesture(&mut term, &mut app, Drag(Left), (grip_x + 3, grip_y + 2));
    assert_eq!(app.dialog_mouse.offset, (3, 2), "it moved by the travel");
    assert_eq!(app.dialog_mouse.grabbed, Some((grip_x + 3, grip_y + 2)));

    gesture(&mut term, &mut app, Up(Left), (grip_x + 3, grip_y + 2));
    assert_eq!(app.dialog_mouse.grabbed, None, "released");

    // A drag with nothing held, and a gesture the dialog has no use for, must
    // both leave the offset alone.
    dispatch_dialog_mouse(
        &mut term,
        &mut app,
        mouse(MouseEventKind::Drag(MouseButton::Left), 1, 1),
    );
    dispatch_dialog_mouse(&mut term, &mut app, mouse(MouseEventKind::Moved, 1, 1));
    assert_eq!(app.dialog_mouse.offset, (3, 2), "unchanged");
}

/// A press outside the dialog does nothing at all: that is what modal means,
/// and acting on the panels behind it would be a surprise.
#[test]
fn a_press_outside_the_dialog_does_nothing() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut term = terminal();
    app.set_overlay(Overlay::QuitConfirm(Confirm::Yes));
    draw(&mut app, &mut term);

    dispatch_dialog_mouse(
        &mut term,
        &mut app,
        mouse(MouseEventKind::Down(MouseButton::Left), 0, 23),
    );

    assert!(matches!(app.overlay, Overlay::QuitConfirm(_)), "still up");
    assert_eq!(app.dialog_mouse.grabbed, None, "and not grabbed");
}

// ─── the pointer through the whole dispatcher, not just the pieces ─────────

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
    // Tall enough that the whole drop-down fits: the Commands menu has 22
    // rows, and a screen no taller than the drop-down would clip the last
    // one — "Go to root" — off the bottom before a click could ever land on it.
    let screen = ratatui::layout::Rect::new(0, 0, 80, 30);
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
    assert_eq!(app.pending_key, None, "the pending key was consumed");
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
