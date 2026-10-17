//! Purpose: A copy or delete stops at the first entry it cannot handle, shows
//! where and why in a dialog, and leaves the rest of the selection alone.

use super::tests_transfer_policies::{app_between, mark, settings};
use super::*;

use crate::models::{OperationFailed, Overlay};
use crate::progress::Operation;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

fn failure(app: &App) -> &OperationFailed {
    let Overlay::OperationFailed(failure) = &app.overlay else {
        panic!("the failure is shown in a dialog");
    };
    failure
}

#[test]
fn a_delete_stops_at_the_first_entry_that_will_not_go() {
    let dir = TempDir::new().expect("test setup");
    let kept = dir.path().join("kept.txt");
    fs::write(&kept, b"kept").expect("test setup");
    let gone = dir.path().join("gone.txt");
    let mut app = app_between(dir.path(), dir.path());
    app.set_overlay(Overlay::DeleteConfirm(vec![gone.clone(), kept.clone()]));

    handle_delete(&mut app).expect("the batch reports");

    assert!(kept.exists(), "nothing after the failure is deleted");
    let failure = failure(&app);
    assert_eq!(failure.operation(), Operation::Delete);
    assert_eq!(failure.failed().path(), gone);
    assert_eq!((failure.done(), failure.left()), (0, 1));
    assert!(
        app.status.to_string().contains("1 error(s)"),
        "{}",
        app.status
    );
}

#[test]
fn a_copy_stops_at_the_first_entry_it_cannot_copy() {
    let from = TempDir::new().expect("test setup");
    let to = TempDir::new().expect("test setup");
    fs::write(from.path().join("a.txt"), b"a").expect("test setup");
    fs::write(from.path().join("b.txt"), b"b").expect("test setup");
    let mut app = app_between(from.path(), to.path());
    mark(&mut app, "a.txt");
    mark(&mut app, "b.txt");
    // Sent where no folder can be made: under a file.
    let blocker = to.path().join("file");
    fs::write(&blocker, b"").expect("test setup");
    let dialog = settings(&blocker.join("sub"));

    copy_with(&mut app, &dialog);

    let failure = failure(&app);
    assert_eq!(failure.operation(), Operation::Copy);
    assert_eq!((failure.done(), failure.left()), (0, 1));
    assert_eq!(
        fs::read_dir(to.path()).unwrap().count(),
        1,
        "nothing was copied"
    );
}

#[test]
fn the_dialog_names_the_entry_and_the_reason_and_closes_on_enter() {
    let dir = TempDir::new().expect("test setup");
    let gone = dir.path().join("gone.txt");
    let mut app = app_between(dir.path(), dir.path());
    app.set_overlay(Overlay::DeleteConfirm(vec![gone]));
    handle_delete(&mut app).expect("the batch reports");

    let mut terminal =
        ratatui::Terminal::new(crate::WritableTestBackend::new(100, 30)).expect("terminal");
    terminal
        .draw(|frame| crate::ui::ui(frame, &mut app))
        .expect("draw");
    let screen = crate::test_support::buffer_text(&terminal);
    assert!(screen.contains("Cannot delete"), "{screen}");
    assert!(screen.contains("gone.txt"), "{screen}");
    assert!(screen.contains("OK"), "{screen}");

    crate::popup::process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
    );
    assert!(
        app.overlay.is_active(),
        "other keys do not reach the panels"
    );
    crate::popup::process_popup_input(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(!app.overlay.is_active());
}

/// The screen with `app`'s dialog on it.
fn screen(app: &mut App) -> String {
    let mut terminal =
        ratatui::Terminal::new(crate::WritableTestBackend::new(100, 30)).expect("terminal");
    terminal
        .draw(|frame| crate::ui::ui(frame, app))
        .expect("draw");
    crate::test_support::buffer_text(&terminal)
}

/// Stop `operation` two entries in, at `x`, which another program holds.
fn stopped_in_use(app: &mut App, dir: &Path, operation: Operation) {
    let failed = crate::status::FailedEntry::new(dir.join("x"), &"refused");
    let remaining = ["x", "y", "z", "w"].map(|name| dir.join(name));
    let stopped = Stop {
        operation,
        failed: &failed,
        denial: Some(crate::models::Denial::InUse),
        done: 2,
        remaining: &remaining,
        base: dir,
        selected: 6,
        settings: None,
    };
    stop(app, &stopped);
}

#[test]
fn every_operation_says_what_it_could_not_do() {
    for (operation, heading) in [
        (Operation::Copy, "Cannot copy"),
        (Operation::Move, "Cannot move"),
        (Operation::Delete, "Cannot delete"),
    ] {
        let dir = TempDir::new().expect("test setup");
        let mut app = app_between(dir.path(), dir.path());
        stopped_in_use(&mut app, dir.path(), operation);
        let screen = screen(&mut app);
        assert!(screen.contains(heading), "{screen}");
        assert!(screen.contains("refused"), "{screen}");
        assert!(screen.contains("open in another program"), "{screen}");
        assert!(
            !screen.contains("administrator"),
            "no retry for a file in use: {screen}"
        );
        assert!(
            screen.contains("Stopped: 2 done, 3 not started"),
            "{screen}"
        );
    }
}
