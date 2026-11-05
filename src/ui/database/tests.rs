//! Purpose: Pin what the explorer draws: its hint when no connection is
//! configured, a connection's name and driver, and Loading while a request
//! is in flight.

use ratatui::{backend::TestBackend, Terminal};

use super::render;
use crate::models::database::{ConnectionSpec, DatabaseExplorer, Driver, Name, Requests, Task};

fn screen(explorer: &DatabaseExplorer) -> String {
    let mut terminal = Terminal::new(TestBackend::new(100, 24)).expect("a terminal");
    terminal
        .draw(|frame| render(frame, explorer))
        .expect("a frame");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

fn shop() -> ConnectionSpec {
    ConnectionSpec {
        name: Name::new("shop").expect("a name"),
        driver: Driver::Sqlite {
            path: "/data/shop.db".into(),
        },
    }
}

#[test]
fn an_explorer_without_connections_shows_its_hint() {
    let explorer = DatabaseExplorer::new(
        Vec::new(),
        Some("No connections: create databases.yaml".to_owned()),
    );
    assert!(screen(&explorer).contains("create databases.yaml"));
}

#[test]
fn a_connection_shows_its_name_and_driver() {
    let explorer = DatabaseExplorer::new(vec![shop()], None);
    let text = screen(&explorer);
    assert!(text.contains("shop"));
    assert!(text.contains("sqlite"));
}

#[test]
fn a_request_in_flight_shows_loading() {
    let mut queue = Requests::default();
    let mut explorer = DatabaseExplorer::new(vec![shop()], None);
    explorer.expand(&mut queue);
    assert!(matches!(
        queue.take().map(|request| request.task),
        Some(Task::Schemas(_))
    ));
    assert!(screen(&explorer).contains("Loading"));
}
