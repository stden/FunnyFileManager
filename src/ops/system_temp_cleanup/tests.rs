//! Purpose: Default-No confirmation is mandatory before a deletion can be queued.
use super::*;
use crate::models::{
    temp_cleanup::{TempOutcome, TempPlan, TempSource},
    Panel,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::{Duration, SystemTime};

#[test]
fn opening_and_cancel_do_not_delete_fixture_files() {
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::write(temp.path().join("keep"), b"keep").unwrap();
    let panel = Panel::from_entries(temp.path().to_owned(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    app.live.temp_cleanup = TempSource::Directory {
        path: temp.path().to_owned(),
        home: None,
    };
    open(&mut app);
    assert!(matches!(app.overlay, Overlay::TempCleanup(_)));
    cancel(&mut app);
    assert_eq!(std::fs::read(temp.path().join("keep")).unwrap(), b"keep");
}

#[test]
fn toggle_and_accept_guard_missing_scanning_and_finished_states() {
    let temp = tempfile::TempDir::new().unwrap();
    let panel = Panel::from_entries(temp.path().to_owned(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    toggle(&mut app);
    accept(&mut app);
    answer(&mut app, Confirm::No);
    assert!(!crate::popup::process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
    ));
    let src = TempSource::Directory {
        path: temp.path().to_owned(),
        home: None,
    };
    let mut scanning = TempCleanupDialog::new(src.clone());
    scanning.state = TempCleanupState::Scanning;
    app.set_overlay(Overlay::TempCleanup(Box::new(scanning)));
    toggle(&mut app);
    answer(&mut app, Confirm::No);
    accept(&mut app);
    let mut finished = TempCleanupDialog::new(src);
    finished.state = TempCleanupState::Finished(TempOutcome::default());
    app.set_overlay(Overlay::TempCleanup(Box::new(finished)));
    accept(&mut app);
    assert!(matches!(app.overlay, Overlay::None));
}

fn plan_with_old_file(temp: &tempfile::TempDir) -> (TempSource, TempPlan) {
    let old = temp.path().join("old.tmp");
    std::fs::write(&old, b"old").unwrap();
    let now = SystemTime::now();
    std::fs::File::options()
        .write(true)
        .open(&old)
        .unwrap()
        .set_modified(now - Duration::from_hours(192))
        .unwrap();
    let src = TempSource::Directory {
        path: temp.path().to_owned(),
        home: None,
    };
    let plan = scan(&src, now).unwrap();
    (src, plan)
}

#[test]
fn preview_and_confirmation_keys_walk_choices_and_queue_delete() {
    let temp = tempfile::TempDir::new().unwrap();
    let panel = Panel::from_entries(temp.path().to_owned(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    let (src, plan) = plan_with_old_file(&temp);
    std::fs::create_dir(temp.path().join("descript.ion")).unwrap();
    app.left_panel.view = crate::models::PanelView::Descriptions;
    finish(&mut app, &TempOutcome::default(), &plan.root);
    assert!(app.status_text().contains("panel refresh failed"));
    let mut dlg = TempCleanupDialog::new(src);
    dlg.state = TempCleanupState::Preview {
        plan,
        focus: TempChoice::Cancel,
    };
    app.set_overlay(Overlay::TempCleanup(Box::new(dlg)));
    for code in [
        KeyCode::Tab,
        KeyCode::Tab,
        KeyCode::Tab,
        KeyCode::Enter,
        KeyCode::Char('z'),
        KeyCode::Char('n'),
        KeyCode::Tab,
        KeyCode::Enter,
        KeyCode::Tab,
        KeyCode::Char('y'),
    ] {
        crate::popup::process_popup_input(&mut app, KeyEvent::new(code, KeyModifiers::NONE));
    }
    assert!(matches!(app.overlay, Overlay::TempCleanup(_)));
}
