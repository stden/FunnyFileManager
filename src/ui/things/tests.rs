//! Unit tests: the Things dialog draws its read in progress and its failure,
//! the machines with their marks and fields, the server's total, and the
//! message a filter that hides everything leaves.

use ratatui::Terminal;

use crate::models::things::{ThingsCatalog, ThingsDialog, ThingsState};
use crate::models::{App, Overlay, Panel, RowMotion};
use crate::test_support::{buffer_text, WritableTestBackend};
use crate::things::{Listing, Machine, OsInfo, Status};
use crate::ui::render_overlays;

fn app_with(state: ThingsState) -> App {
    let panel = Panel::from_entries(std::env::temp_dir(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_overlay(Overlay::Things(Box::new(ThingsDialog::new(state))));
    app
}

fn draw(app: &mut App) -> String {
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    terminal.draw(|f| render_overlays(f, app)).expect("draw");
    buffer_text(&terminal)
}

fn web() -> Machine {
    Machine {
        id: "1".to_owned(),
        name: "web-01".to_owned(),
        hostname: "web-01.example.com".to_owned(),
        status: Status::Online,
        os_info: Some(OsInfo {
            name: "Ubuntu".to_owned(),
            version: "24.04".to_owned(),
        }),
        environment: Some("PROD".to_owned()),
        ..Machine::default()
    }
}

fn db() -> Machine {
    Machine {
        id: "2".to_owned(),
        name: "db-01".to_owned(),
        ip_address: Some("10.0.0.9".to_owned()),
        ..Machine::default()
    }
}

fn shown(machines: Vec<Machine>, total: Option<usize>) -> App {
    app_with(ThingsState::Ready(ThingsCatalog::new(Listing::new(
        machines, total,
    ))))
}

fn catalog_mut(app: &mut App) -> &mut ThingsCatalog {
    let Overlay::Things(dialog) = &mut app.overlay else {
        panic!("the dialog is open");
    };
    let ThingsState::Ready(catalog) = &mut dialog.state else {
        panic!("the machines are shown");
    };
    catalog
}

#[test]
fn while_the_read_is_out_the_dialog_says_it_is_loading() {
    let mut app = app_with(ThingsState::Loading);
    assert!(draw(&mut app).contains("Loading Things..."));
}

#[test]
fn a_setup_problem_is_shown_in_the_words_it_was_given() {
    let mut app = app_with(ThingsState::Setup(
        "Set THINGS_URL to browse Things".to_owned(),
    ));
    assert!(draw(&mut app).contains("Set THINGS_URL to browse Things"));
}

#[test]
fn each_machine_shows_its_name_host_system_environment_and_state() {
    let mut app = shown(vec![web(), db()], None);
    let screen = draw(&mut app);
    for text in [
        "web-01",
        "web-01.example.com",
        "Ubuntu 24.04",
        "PROD",
        "online",
        "db-01",
        "10.0.0.9",
    ] {
        assert!(screen.contains(text), "{text} is not on screen\n{screen}");
    }
}

#[test]
fn the_title_says_how_many_the_server_has_when_more_were_read() {
    let mut app = shown(vec![web(), db()], Some(120));
    assert!(draw(&mut app).contains("showing 2 of 120"));
}

#[test]
fn a_marked_row_carries_a_star_and_an_unmarked_one_does_not() {
    let mut app = shown(vec![db(), web()], None);
    catalog_mut(&mut app).move_cursor(RowMotion::Last);
    catalog_mut(&mut app).toggle_mark();
    let screen = draw(&mut app);
    assert!(screen.contains("* web-01"), "{screen}");
    assert!(!screen.contains("* db-01"), "{screen}");
}

#[test]
fn a_filter_that_hides_every_machine_says_so() {
    let mut app = shown(vec![web()], None);
    catalog_mut(&mut app).type_filter('z');
    assert!(draw(&mut app).contains("No machine matches the filter"));
}

#[test]
fn an_empty_listing_says_it_listed_no_machines() {
    let mut app = shown(Vec::new(), None);
    assert!(draw(&mut app).contains("Things listed no machines"));
}

#[test]
fn a_control_character_in_a_machine_field_is_not_drawn() {
    let machine = Machine {
        name: "web\u{1b}[31m-01".to_owned(),
        ..web()
    };
    let mut app = shown(vec![machine], None);
    let screen = draw(&mut app);
    assert!(!screen.contains('\u{1b}'), "{screen}");
    assert!(screen.contains("web\u{FFFD}[31m-01"), "{screen}");
}

#[test]
fn an_ordinary_name_with_spaces_and_letters_is_drawn_unchanged() {
    let machine = Machine {
        name: "Büro Nord 2".to_owned(),
        ..web()
    };
    let mut app = shown(vec![machine], None);
    assert!(draw(&mut app).contains("Büro Nord 2"));
}
