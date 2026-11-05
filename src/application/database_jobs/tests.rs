//! Purpose: The database job runs off the UI thread: its answer reaches the
//! dialog that asked for it, and the answer of a dialog closed while it ran
//! is dropped.

use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::DatabaseJobs;
use crate::models::database::{ConnectionSpec, DatabaseExplorer, Driver, Load, Name};
use crate::models::{App, Overlay, Panel};
use crate::popup::process_popup_input;

fn app_with_database(temp: &tempfile::TempDir) -> App {
    let path = temp.path().join("shop.db");
    let writer = rusqlite::Connection::open(&path).expect("the database");
    writer
        .execute_batch("CREATE TABLE items (id INTEGER PRIMARY KEY);")
        .expect("schema");
    let spec = ConnectionSpec {
        name: Name::new("shop").expect("a name"),
        driver: Driver::Sqlite { path },
    };
    let mut app = App::with_panels(
        Panel::from_entries(temp.path().to_path_buf(), Vec::new()),
        Panel::new(temp.path().to_path_buf()).expect("local panel"),
    );
    app.set_overlay(Overlay::Database(Box::new(DatabaseExplorer::new(
        vec![spec],
        None,
    ))));
    app
}

fn loading(app: &App) -> bool {
    matches!(&app.overlay, Overlay::Database(explorer) if explorer.is_loading())
}

fn pump_until_idle(jobs: &mut DatabaseJobs, app: &mut App) {
    for _ in 0..2000 {
        jobs.pump(app);
        if app.database_requests.is_idle() && !loading(app) {
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("the database job did not finish");
}

#[test]
fn the_job_delivers_its_answer_to_the_dialog_that_asked() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app_with_database(&temp);
    let mut jobs = DatabaseJobs::default();
    assert!(process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)
    ));
    pump_until_idle(&mut jobs, &mut app);
    let Overlay::Database(explorer) = &app.overlay else {
        panic!("the explorer is still open")
    };
    assert!(matches!(explorer.connections()[0].schemas, Load::Ready(_)));
    assert_eq!(explorer.rows().len(), 2);
}

#[test]
fn an_answer_for_a_dialog_closed_while_it_ran_is_dropped() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app_with_database(&temp);
    let mut jobs = DatabaseJobs::default();
    assert!(process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)
    ));
    jobs.pump(&mut app);
    app.close_overlay();
    pump_until_idle(&mut jobs, &mut app);
    assert!(matches!(app.overlay, Overlay::None));
}
