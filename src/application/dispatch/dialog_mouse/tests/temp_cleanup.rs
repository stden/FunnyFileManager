//! Mouse actions must preserve the confirmation required for temporary-file deletion.
use super::*;
use crate::models::temp_cleanup::{TempChoice, TempCleanupState, TempError, TempSource};
use crate::models::TempCleanupDialog;
use std::{
    fs,
    time::{Duration, SystemTime},
};

fn preview(app: &mut App, dir: &TempDir, with_file: bool) {
    let root = dir.path().join("temp");
    fs::create_dir(&root).unwrap();
    let now = SystemTime::now();
    if with_file {
        let path = root.join("old");
        fs::write(&path, b"old").unwrap();
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(now - Duration::from_hours(192))
            .unwrap();
    }
    let source = TempSource::Directory {
        path: root,
        home: None,
    };
    let plan = crate::ops::system_temp_cleanup::scan(&source, now).unwrap();
    let mut dialog = TempCleanupDialog::new(source);
    dialog.state = TempCleanupState::Preview {
        plan,
        focus: TempChoice::Cancel,
    };
    app.set_overlay(Overlay::TempCleanup(Box::new(dialog)));
}

#[test]
fn clean_button_opens_confirmation_even_when_cancel_was_focused() {
    let dir = TempDir::new().unwrap();
    let mut app = app_at(&dir);
    preview(&mut app, &dir, true);
    click_text(&mut terminal(), &mut app, "Clean up");
    assert!(matches!(app.overlay, Overlay::TempCleanup(ref dialog)
        if matches!(dialog.state, TempCleanupState::Confirm { answer: Confirm::No, .. })));
    assert!(dir.path().join("temp/old").exists());
}

#[test]
fn clicking_no_returns_to_preview_without_deletion() {
    let dir = TempDir::new().unwrap();
    let mut app = app_at(&dir);
    preview(&mut app, &dir, true);
    let mut term = terminal();
    click_text(&mut term, &mut app, "Clean up");
    draw(&mut app, &mut term);
    let &(_, left, _) = app
        .dialog_mouse
        .buttons
        .iter()
        .find(|button| button.0 == 1)
        .unwrap();
    let row = app.dialog_mouse.buttons_y;
    gesture(
        &mut term,
        &mut app,
        MouseEventKind::Down(MouseButton::Left),
        (left, row),
    );
    assert!(matches!(app.overlay, Overlay::TempCleanup(ref dialog)
        if matches!(dialog.state, TempCleanupState::Preview { .. })));
    assert_eq!(fs::read(dir.path().join("temp/old")).unwrap(), b"old");
}

#[test]
fn clicking_yes_queues_only_the_confirmed_plan() {
    let dir = TempDir::new().unwrap();
    let mut app = app_at(&dir);
    preview(&mut app, &dir, true);
    let mut term = terminal();
    click_text(&mut term, &mut app, "Clean up");
    click_text(&mut term, &mut app, "Yes");
    assert!(matches!(app.overlay, Overlay::TempCleanup(ref dialog)
        if matches!(dialog.state, TempCleanupState::QueuedDelete(_))));
    assert!(dir.path().join("temp/old").exists());
}

#[test]
fn cancel_button_closes_preview_without_touching_files() {
    let dir = TempDir::new().unwrap();
    let mut app = app_at(&dir);
    preview(&mut app, &dir, true);
    click_text(&mut terminal(), &mut app, "Cancel");
    assert!(matches!(app.overlay, Overlay::None));
    assert!(dir.path().join("temp/old").exists());
}

#[test]
fn empty_preview_rejects_cleanup_and_unknown_buttons() {
    let dir = TempDir::new().unwrap();
    let mut app = app_at(&dir);
    preview(&mut app, &dir, false);
    assert!(dialog_button_key(&mut app, 0).is_none());
    assert!(dialog_button_key(&mut app, 2).is_none());
    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    assert!(matches!(app.overlay, Overlay::TempCleanup(ref dialog)
        if matches!(dialog.state, TempCleanupState::Preview { focus: TempChoice::Cancel, .. })));
}

#[test]
fn failure_close_button_sends_escape_and_unknown_button_does_nothing() {
    let dir = TempDir::new().unwrap();
    let mut app = app_at(&dir);
    let mut dialog = TempCleanupDialog::new(TempSource::Unavailable);
    dialog.state = TempCleanupState::Failed(TempError::Worker);
    app.set_overlay(Overlay::TempCleanup(Box::new(dialog)));
    assert!(dialog_button_key(&mut app, 1).is_none());
    click_text(&mut terminal(), &mut app, "Close");
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn failed_cleanup_explains_each_failure_and_can_be_closed() {
    for (error, message) in [
        (TempError::Unavailable, "unavailable in this session"),
        (TempError::Scan, "Cannot scan the temporary directory"),
        (TempError::Clock, "Cannot determine the temporary-file age"),
        (TempError::Limit, "exceeded its safety limit"),
    ] {
        let dir = TempDir::new().unwrap();
        let mut app = app_at(&dir);
        let mut dialog = TempCleanupDialog::new(TempSource::Unavailable);
        dialog.state = TempCleanupState::Failed(error);
        app.set_overlay(Overlay::TempCleanup(Box::new(dialog)));
        let mut term = terminal();
        draw(&mut app, &mut term);
        let screen = crate::test_support::buffer_string(term.backend().buffer());
        assert!(screen.contains(message), "{screen}");
        click_text(&mut term, &mut app, "Close");
        assert!(matches!(app.overlay, Overlay::None));
    }
}

#[test]
fn nonexistent_confirmation_button_does_not_change_the_answer() {
    let dir = TempDir::new().unwrap();
    let mut app = app_at(&dir);
    preview(&mut app, &dir, true);
    click_text(&mut terminal(), &mut app, "Clean up");
    assert!(dialog_button_key(&mut app, 2).is_none());
    assert!(matches!(app.overlay, Overlay::TempCleanup(ref dialog)
        if matches!(dialog.state, TempCleanupState::Confirm { answer: Confirm::No, .. })));
    assert!(dir.path().join("temp/old").exists());
}
