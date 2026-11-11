//! Purpose: Pin the explorer's keys as the dialog receives them: arrows, Enter,
//! Tab and c, Esc stepping back before it closes, F10, and a key nothing is
//! bound to, which the dialog swallows.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::database::explorer::Expansion;
use crate::models::database::{
    Column, ConnectionSpec, DatabaseExplorer, Driver, KeyRole, Name, Nullability, Reply, Schema,
    Table, TableKind, Task,
};
use crate::models::{App, Overlay, Panel};
use crate::popup::process_popup_input;

mod commands;

fn app(temp: &tempfile::TempDir) -> App {
    let mut app = App::with_panels(
        Panel::from_entries(temp.path().to_path_buf(), Vec::new()),
        Panel::new(temp.path().to_path_buf()).expect("local panel"),
    );
    let spec = ConnectionSpec {
        name: Name::new("local").expect("a name"),
        driver: Driver::Sqlite {
            path: temp.path().join("x.db"),
        },
    };
    app.set_overlay(Overlay::Database(Box::new(DatabaseExplorer::new(
        vec![spec],
        None,
    ))));
    app
}

fn press(app: &mut App, code: KeyCode) {
    assert!(process_popup_input(
        app,
        KeyEvent::new(code, KeyModifiers::NONE)
    ));
}

fn explorer(app: &App) -> &DatabaseExplorer {
    let Overlay::Database(explorer) = &app.overlay else {
        panic!("the explorer is open")
    };
    explorer
}

/// Answer the queued request with `reply`; the task it asked for comes back.
fn answer_queued(app: &mut App, reply: Reply) -> Task {
    let request = app.database_requests.take().expect("a queued request");
    assert!(app.database_requests.finish(request.id));
    let Overlay::Database(explorer) = &mut app.overlay else {
        panic!("the explorer is open")
    };
    explorer.receive(request.id, Ok(reply));
    request.task
}

fn with_schemas(app: &mut App) {
    press(app, KeyCode::Right);
    answer_queued(
        app,
        Reply::Schemas(vec![Schema {
            name: Name::new("main").expect("a name"),
            tables: vec![Table {
                name: Name::new("items").expect("a name"),
                kind: TableKind::Table,
            }],
        }]),
    );
    press(app, KeyCode::Down);
    press(app, KeyCode::Right);
    press(app, KeyCode::Down);
}

#[test]
fn right_opens_the_connection_and_queues_its_schemas() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app(&temp);
    press(&mut app, KeyCode::Right);
    assert_eq!(
        explorer(&app).connections()[0].expansion,
        Expansion::Expanded
    );
    let request = app.database_requests.take().expect("a queued request");
    assert!(matches!(request.task, Task::Schemas(_)));
}

#[test]
fn f10_closes_the_dialog() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app(&temp);
    press(&mut app, KeyCode::F(10));
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn an_unbound_key_is_swallowed_and_keeps_the_dialog_open() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app(&temp);
    press(&mut app, KeyCode::Char('z'));
    assert!(matches!(app.overlay, Overlay::Database(_)));
}

#[test]
fn enter_on_a_table_opens_rows_and_esc_steps_back_before_closing() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app(&temp);
    with_schemas(&mut app);
    press(&mut app, KeyCode::Enter);
    assert!(matches!(
        explorer(&app).view(),
        crate::models::database::View::Rows(_)
    ));
    press(&mut app, KeyCode::Esc);
    assert!(matches!(
        explorer(&app).view(),
        crate::models::database::View::Tree
    ));
    assert!(matches!(app.overlay, Overlay::Database(_)));
    press(&mut app, KeyCode::Esc);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn tab_and_c_both_open_the_columns_of_the_table_under_the_cursor() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app(&temp);
    with_schemas(&mut app);
    press(&mut app, KeyCode::Tab);
    assert!(matches!(
        explorer(&app).view(),
        crate::models::database::View::Columns(_)
    ));
    answer_queued(
        &mut app,
        Reply::Columns(vec![Column {
            name: "id".to_owned(),
            data_type: "INTEGER".to_owned(),
            key: KeyRole::Primary,
            nullability: Nullability::NotNull,
        }]),
    );
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('c'));
    assert!(matches!(
        explorer(&app).view(),
        crate::models::database::View::Columns(_)
    ));
}
