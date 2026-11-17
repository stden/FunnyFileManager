//! Purpose: The keys the dialog tests do not reach elsewhere (Up, Left, the page
//! keys, F5), every command through `invoke` as the automation interface sends
//! it, and what a command does when no explorer is open.

use super::*;
use crate::action::database::DatabaseCommand;
use crate::models::database::{Page, PageEnd, View};
use crate::popup::database::{invoke, process};

fn rows_reply(end: PageEnd) -> Reply {
    Reply::Rows(Page {
        columns: vec!["id".to_owned()],
        rows: vec![vec!["1".to_owned()]],
        end,
    })
}

#[test]
fn up_moves_the_cursor_back_up_the_tree() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app(&temp);
    with_schemas(&mut app);
    assert_eq!(explorer(&app).cursor(), 2);
    press(&mut app, KeyCode::Up);
    assert_eq!(explorer(&app).cursor(), 1);
}

#[test]
fn left_closes_the_open_row_and_otherwise_goes_to_its_parent() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app(&temp);
    with_schemas(&mut app);
    press(&mut app, KeyCode::Left);
    assert_eq!(explorer(&app).cursor(), 1, "from a table to its schema");
    assert_eq!(explorer(&app).rows().len(), 3);
    press(&mut app, KeyCode::Left);
    assert_eq!(explorer(&app).rows().len(), 2, "the schema closes");
}

#[test]
fn page_keys_turn_the_pages_of_a_table_that_said_more_follow() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app(&temp);
    with_schemas(&mut app);
    press(&mut app, KeyCode::Enter);
    let first = answer_queued(&mut app, rows_reply(PageEnd::More));
    assert!(matches!(first, Task::Rows { offset: 0, .. }));

    press(&mut app, KeyCode::PageDown);
    let next = answer_queued(&mut app, rows_reply(PageEnd::Last));
    assert!(matches!(next, Task::Rows { offset: 200, .. }));

    press(&mut app, KeyCode::PageUp);
    let back = answer_queued(&mut app, rows_reply(PageEnd::More));
    assert!(matches!(back, Task::Rows { offset: 0, .. }));
}

#[test]
fn f5_reads_the_metadata_of_the_connection_again() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app(&temp);
    with_schemas(&mut app);
    press(&mut app, KeyCode::F(5));
    let request = app.database_requests.take().expect("a queued request");
    assert!(matches!(request.task, Task::Schemas(ref found) if found.name.as_str() == "local"));
}

#[test]
fn invoke_moves_and_opens_the_tree_as_its_keys_would() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app(&temp);
    with_schemas(&mut app);
    invoke(&mut app, DatabaseCommand::Up);
    assert_eq!(explorer(&app).cursor(), 1);
    invoke(&mut app, DatabaseCommand::Down);
    assert_eq!(explorer(&app).cursor(), 2);
    invoke(&mut app, DatabaseCommand::Collapse);
    assert_eq!(explorer(&app).cursor(), 1);
    invoke(&mut app, DatabaseCommand::Expand);
    assert_eq!(explorer(&app).rows().len(), 3);

    invoke(&mut app, DatabaseCommand::Down);
    invoke(&mut app, DatabaseCommand::Columns);
    assert!(matches!(explorer(&app).view(), View::Columns(_)));
    let asked = answer_queued(&mut app, Reply::Columns(Vec::new()));
    assert!(matches!(asked, Task::Columns { .. }));

    invoke(&mut app, DatabaseCommand::Back);
    invoke(&mut app, DatabaseCommand::Reload);
    let asked = answer_queued(&mut app, Reply::Schemas(Vec::new()));
    assert!(matches!(asked, Task::Schemas(_)));
}

#[test]
fn invoke_opens_and_pages_the_rows_as_its_keys_would() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app(&temp);
    with_schemas(&mut app);
    invoke(&mut app, DatabaseCommand::Open);
    assert!(matches!(explorer(&app).view(), View::Rows(_)));
    let first = answer_queued(&mut app, rows_reply(PageEnd::More));
    assert!(matches!(first, Task::Rows { offset: 0, .. }));

    invoke(&mut app, DatabaseCommand::PageDown);
    let next = answer_queued(&mut app, rows_reply(PageEnd::Last));
    assert!(matches!(next, Task::Rows { offset: 200, .. }));
    invoke(&mut app, DatabaseCommand::PageUp);
    let back = answer_queued(&mut app, rows_reply(PageEnd::Last));
    assert!(matches!(back, Task::Rows { offset: 0, .. }));
}

#[test]
fn invoke_back_steps_out_of_a_view_and_then_closes_like_close() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app(&temp);
    with_schemas(&mut app);
    invoke(&mut app, DatabaseCommand::Open);
    invoke(&mut app, DatabaseCommand::Back);
    assert!(matches!(explorer(&app).view(), View::Tree));
    assert!(matches!(app.overlay, Overlay::Database(_)));
    invoke(&mut app, DatabaseCommand::Back);
    assert!(matches!(app.overlay, Overlay::None));

    let mut app = self::app(&temp);
    invoke(&mut app, DatabaseCommand::Close);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn a_command_with_no_explorer_open_does_nothing() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app(&temp);
    app.close_overlay();
    invoke(&mut app, DatabaseCommand::Expand);
    invoke(&mut app, DatabaseCommand::Close);
    assert!(matches!(app.overlay, Overlay::None));
    assert!(app.database_requests.is_idle());

    assert!(!process(&mut app, KeyCode::Right, KeyModifiers::NONE));
    assert!(app.database_requests.is_idle());
}
