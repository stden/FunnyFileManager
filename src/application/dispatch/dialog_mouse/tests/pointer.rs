//! Purpose: the pointer routed through `dispatch_dialog_mouse`: wheel, drag,
//! and presses outside the dialog.

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
    dispatch_dialog_mouse(&mut term, &mut app, mouse(Drag(Left), 1, 1));
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
