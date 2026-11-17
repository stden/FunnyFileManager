//! Purpose: A preview queues deletion only after its default-No question is answered Yes.
use super::*;

/// An app showing the preview of a plan with one old file, focused on Cancel.
fn preview() -> (tempfile::TempDir, App) {
    let temp = tempfile::TempDir::new().unwrap();
    let panel = Panel::from_entries(temp.path().to_owned(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    let (src, plan) = plan_with_old_file(&temp);
    let mut dialog = TempCleanupDialog::new(src);
    dialog.state = TempCleanupState::Preview {
        plan,
        focus: TempChoice::Cancel,
    };
    app.set_overlay(Overlay::TempCleanup(Box::new(dialog)));
    (temp, app)
}

/// The focused choice of the preview on screen; `None` without one.
fn preview_focus(app: &App) -> Option<TempChoice> {
    let Overlay::TempCleanup(dialog) = &app.overlay else {
        return None;
    };
    match &dialog.state {
        TempCleanupState::Preview { focus, .. } => Some(*focus),
        _ => None,
    }
}

#[test]
fn toggling_the_preview_twice_returns_to_cancel() {
    let (_temp, mut app) = preview();
    toggle(&mut app);
    assert_eq!(preview_focus(&app), Some(TempChoice::Clean));
    toggle(&mut app);
    assert_eq!(preview_focus(&app), Some(TempChoice::Cancel));
}

#[test]
fn selecting_cleanup_still_requires_default_no_confirmation_before_queueing() {
    let (temp, mut app) = preview();
    toggle(&mut app);
    accept(&mut app);
    assert!(matches!(&app.overlay, Overlay::TempCleanup(dialog)
        if matches!(dialog.state, TempCleanupState::Confirm { answer: Confirm::No, .. })));
    toggle(&mut app);
    toggle(&mut app);
    answer(&mut app, Confirm::Yes);
    assert!(matches!(&app.overlay, Overlay::TempCleanup(dialog)
        if matches!(dialog.state, TempCleanupState::QueuedDelete(_))));
    assert_eq!(std::fs::read(temp.path().join("old.tmp")).unwrap(), b"old");
}

#[test]
fn declining_confirmation_returns_to_the_safe_cancel_choice() {
    let (temp, mut app) = preview();
    toggle(&mut app);
    accept(&mut app);
    answer(&mut app, Confirm::No);
    assert!(matches!(&app.overlay, Overlay::TempCleanup(dialog)
        if matches!(dialog.state, TempCleanupState::Preview { focus: TempChoice::Cancel, .. })));
    accept(&mut app);
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(std::fs::read(temp.path().join("old.tmp")).unwrap(), b"old");
}

#[test]
fn cleanup_actions_ignore_other_overlays_and_pending_workers() {
    let (_temp, mut app) = preview();
    app.close_overlay();
    toggle(&mut app);
    answer(&mut app, Confirm::Yes);
    accept(&mut app);
    assert!(matches!(app.overlay, Overlay::None));
    open(&mut app);
    toggle(&mut app);
    answer(&mut app, Confirm::Yes);
    accept(&mut app);
    assert!(matches!(&app.overlay, Overlay::TempCleanup(dialog)
        if matches!(dialog.state, TempCleanupState::QueuedScan(_))));
}
