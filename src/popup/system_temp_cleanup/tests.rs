//! Purpose: Temp cleanup popup handler returns false when another overlay is
//! active, and its confirmation queues cleanup only after an explicit,
//! unmodified answer.

use super::process;
use crate::models::temp_cleanup::{TempChoice, TempCleanupState, TempSource};
use crate::models::{App, Confirm, Overlay, Panel, TempCleanupDialog};
use crossterm::event::{KeyCode, KeyModifiers};
use std::path::PathBuf;

#[test]
fn process_returns_false_when_temp_cleanup_is_closed() {
    let empty = || Panel::from_entries(PathBuf::from("/tmp"), Vec::new());
    let mut app = App::with_panels(empty(), empty());
    assert!(!process(&mut app, KeyCode::Esc, KeyModifiers::NONE));
}

/// The dialog asking to delete an empty folder's plan, on its default No.
fn confirmation() -> (tempfile::TempDir, App) {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let source = TempSource::Directory {
        path: temp.path().to_owned(),
        home: None,
    };
    let now = std::time::SystemTime::now();
    let plan = crate::ops::system_temp_cleanup::scan(&source, now).expect("safe preview");
    let panel = Panel::from_entries(temp.path().to_owned(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    let mut dialog = TempCleanupDialog::new(source);
    dialog.state = TempCleanupState::Confirm {
        plan,
        answer: Confirm::No,
    };
    app.set_overlay(Overlay::TempCleanup(Box::new(dialog)));
    (temp, app)
}

#[test]
fn the_no_hotkey_returns_to_the_preview_with_cancel_selected() {
    let (_temp, mut app) = confirmation();
    assert!(process(&mut app, KeyCode::Char('N'), KeyModifiers::SHIFT));
    assert!(matches!(&app.overlay, Overlay::TempCleanup(dialog)
        if matches!(dialog.state, TempCleanupState::Preview { focus: TempChoice::Cancel, .. })));
}

#[test]
fn the_yes_hotkey_queues_the_confirmed_plan() {
    let (_temp, mut app) = confirmation();
    assert!(process(&mut app, KeyCode::Char('y'), KeyModifiers::NONE));
    assert!(matches!(&app.overlay, Overlay::TempCleanup(dialog)
        if matches!(dialog.state, TempCleanupState::QueuedDelete(_))));
}

#[test]
fn modified_confirmation_letters_leave_the_default_no_answer() {
    let (_temp, mut app) = confirmation();
    assert!(process(&mut app, KeyCode::Char('y'), KeyModifiers::CONTROL));
    assert!(matches!(&app.overlay, Overlay::TempCleanup(dialog)
        if matches!(dialog.state, TempCleanupState::Confirm { answer: Confirm::No, .. })));
}
