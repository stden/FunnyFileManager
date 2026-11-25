//! Purpose: Pin how an answer reaches the explorer that asked: a failure is shown
//! in the status bar and kept on the node that asked, a success leaves the status
//! bar alone, and an answer with no dialog open changes nothing.

use super::apply;
use crate::models::database::{DatabaseExplorer, Load, Reply, RequestId};
use crate::models::{App, Overlay, Panel};
use crate::ops::database::fixtures::sqlite_spec;
use crate::ops::database::DbError;
use crate::popup::process_popup_input;
use crate::status::Status;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// An app whose open explorer has asked to load the schemas of its one
/// connection; also the id of that request.
fn asking() -> (App, RequestId) {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    let mut app = App::from_panels(panel.clone(), panel);
    let spec = sqlite_spec(temp.path().join("shop.db"));
    app.set_overlay(Overlay::Database(Box::new(DatabaseExplorer::new(
        vec![spec],
        None,
    ))));
    assert!(process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)
    ));
    let request = app.database_requests.take().expect("the queued request");
    (app, request.id)
}

fn explorer(app: &App) -> &DatabaseExplorer {
    let Overlay::Database(explorer) = &app.overlay else {
        panic!("the explorer is open")
    };
    explorer
}

#[test]
fn a_failed_answer_is_shown_and_kept_on_the_node_that_asked() {
    let (mut app, id) = asking();
    apply(
        &mut app,
        id,
        Err(DbError::Driver("no such file".to_owned())),
    );
    assert_eq!(app.status, Status::Error("no such file".to_owned()));
    assert!(!explorer(&app).is_loading());
    assert!(matches!(
        &explorer(&app).connections()[0].schemas,
        Load::Failed(text) if text == "no such file"
    ));
}

#[test]
fn a_missing_password_is_shown_as_the_variable_that_is_unset() {
    let (mut app, id) = asking();
    apply(
        &mut app,
        id,
        Err(DbError::MissingPassword("DB_PASSWORD".to_owned())),
    );
    assert_eq!(
        app.status,
        Status::Error("environment variable DB_PASSWORD is not set".to_owned())
    );
}

#[test]
fn a_good_answer_fills_the_node_and_leaves_the_status_bar_alone() {
    let (mut app, id) = asking();
    apply(&mut app, id, Ok(Reply::Schemas(Vec::new())));
    assert_eq!(app.status, Status::KeyHints);
    assert!(matches!(
        explorer(&app).connections()[0].schemas,
        Load::Ready(_)
    ));
}

#[test]
fn an_answer_with_no_dialog_open_changes_nothing() {
    let (mut app, id) = asking();
    app.close_overlay();
    apply(&mut app, id, Err(DbError::Driver("late".to_owned())));
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.status, Status::KeyHints);
}
