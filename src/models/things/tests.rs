//! Unit tests: the catalog's cursor, filter and marks, what an import takes,
//! the total it reports, and the dialog handing out its waiting read once.

use super::{ThingsCatalog, ThingsDialog, ThingsState};
use crate::models::RowMotion;
use crate::things::{Config, Listing, Machine};

fn catalog(names: &[&str]) -> ThingsCatalog {
    let machines = names
        .iter()
        .map(|name| Machine {
            id: format!("id-{name}"),
            name: (*name).to_owned(),
            hostname: format!("{name}.example.com"),
            ..Machine::default()
        })
        .collect();
    ThingsCatalog::new(Listing::new(machines, None))
}

fn chosen_names(catalog: &ThingsCatalog) -> Vec<&str> {
    catalog
        .chosen()
        .into_iter()
        .map(|machine| machine.name.as_str())
        .collect()
}

fn marked_names(catalog: &ThingsCatalog) -> Vec<&str> {
    catalog
        .shown()
        .into_iter()
        .filter(|(marked, _)| *marked)
        .map(|(_, machine)| machine.name.as_str())
        .collect()
}

#[test]
fn the_cursor_moves_within_the_shown_rows_and_stops_at_each_end() {
    let mut catalog = catalog(&["a", "b", "c"]);
    for _ in 0..3 {
        catalog.move_cursor(RowMotion::Next);
    }
    assert_eq!(catalog.cursor_index(), 2);
    catalog.move_cursor(RowMotion::Previous);
    assert_eq!(catalog.cursor_index(), 1);
    catalog.move_cursor(RowMotion::Last);
    assert_eq!(catalog.cursor_index(), 2);
    catalog.move_cursor(RowMotion::First);
    assert_eq!(catalog.cursor_index(), 0);
}

#[test]
fn space_marks_the_row_under_the_cursor_and_a_second_press_unmarks_it() {
    let mut catalog = catalog(&["a", "b"]);
    catalog.move_cursor(RowMotion::Next);
    catalog.toggle_mark();
    assert_eq!(marked_names(&catalog), ["b"]);
    catalog.toggle_mark();
    assert_eq!(marked_names(&catalog), Vec::<&str>::new());
}

#[test]
fn with_no_marks_an_import_takes_every_shown_row() {
    let mut catalog = catalog(&["db-01", "web-01", "web-02"]);
    for ch in "web".chars() {
        catalog.type_filter(ch);
    }
    assert_eq!(chosen_names(&catalog), ["web-01", "web-02"]);
}

#[test]
fn marked_rows_win_even_when_the_filter_hides_them() {
    let mut catalog = catalog(&["db-01", "web-01", "web-02"]);
    catalog.move_cursor(RowMotion::Last);
    catalog.toggle_mark();
    catalog.type_filter('d');
    assert_eq!(chosen_names(&catalog), ["web-02"]);
}

#[test]
fn a_filter_edit_starts_the_list_again_at_its_top() {
    let mut catalog = catalog(&["a", "b", "c"]);
    catalog.move_cursor(RowMotion::Last);
    catalog.type_filter('x');
    assert_eq!(catalog.cursor_index(), 0);
    catalog.erase_filter();
    assert_eq!(catalog.filter(), "");
    assert_eq!(chosen_names(&catalog), ["a", "b", "c"]);
}

#[test]
fn the_server_total_is_reported_only_when_more_machines_exist_than_were_read() {
    let machines = vec![Machine {
        name: "a".to_owned(),
        ..Machine::default()
    }];
    let more = ThingsCatalog::new(Listing::new(machines.clone(), Some(120)));
    assert_eq!(more.machine_count(), 1);
    assert_eq!(more.unshown_total(), Some(120));
    let all = ThingsCatalog::new(Listing::new(machines, Some(1)));
    assert_eq!(all.unshown_total(), None);
}

#[test]
fn a_waiting_read_is_handed_out_once() {
    let config = Config::from_values(
        Some("https://things.example.com".to_owned()),
        Some("test-token-123".to_owned()),
    )
    .expect("settings");
    let mut dialog = ThingsDialog::new(ThingsState::Queued(config));
    assert!(dialog.start().is_some());
    assert!(matches!(dialog.state, ThingsState::Loading));
    assert!(dialog.start().is_none());
}
