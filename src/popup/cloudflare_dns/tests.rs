//! Purpose: The dialog's keys. The lists move and load, Enter edits, `a` and `r` ask for a request,
//! Esc steps back and then closes, and nothing moves while a request is in flight.

use crate::models::cloudflare_dns::{
    CloudflareDnsPanel, Focus, FormField, Proxy, Record, RecordForm, RecordId, Request, Target,
    Ttl, View, ViewKind, Zone, ZoneId,
};
use crate::models::{App, Overlay, Panel};
use crate::popup::process_popup_input;
use crossterm::event::{KeyCode, KeyEvent};

const ZONE_A: &str = "0000000000000000000000000000000a";
const ZONE_B: &str = "0000000000000000000000000000000c";

fn app(temp: &tempfile::TempDir) -> App {
    let panel = Panel::from_entries(temp.path().to_path_buf(), Vec::new());
    App::with_panels(panel.clone(), panel)
}

/// A dialog with two zones and one record in the first, idle.
fn idle_dialog() -> CloudflareDnsPanel {
    let mut panel = CloudflareDnsPanel::new();
    panel.set_zones(vec![
        Zone {
            id: ZoneId::new(ZONE_A),
            name: "example.com".to_owned(),
        },
        Zone {
            id: ZoneId::new(ZONE_B),
            name: "example.net".to_owned(),
        },
    ]);
    panel.set_records(vec![Record {
        id: RecordId::new("0000000000000000000000000000000b"),
        kind: "A".to_owned(),
        name: "www.example.com".to_owned(),
        content: "203.0.113.5".to_owned(),
        ttl: Ttl::Seconds(300),
        proxy: Proxy::Off,
    }]);
    panel
}

fn open_with(app: &mut App, panel: CloudflareDnsPanel) {
    app.set_overlay(Overlay::CloudflareDns(Box::new(panel)));
}

fn press(app: &mut App, code: KeyCode) -> bool {
    process_popup_input(app, KeyEvent::from(code))
}

fn dialog(app: &App) -> &CloudflareDnsPanel {
    let Overlay::CloudflareDns(panel) = &app.overlay else {
        panic!("the dialog");
    };
    panel
}

#[test]
fn escape_steps_back_from_a_form_and_then_closes_the_dialog() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    open_with(&mut app, idle_dialog());
    assert!(press(&mut app, KeyCode::Tab));
    assert!(press(&mut app, KeyCode::Enter));
    assert_eq!(dialog(&app).view.kind(), ViewKind::Edit);
    press(&mut app, KeyCode::Esc);
    assert_eq!(dialog(&app).view.kind(), ViewKind::Browse);
    press(&mut app, KeyCode::Esc);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn the_arrows_move_the_zones_and_load_the_records_of_the_zone_reached() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    open_with(&mut app, idle_dialog());
    press(&mut app, KeyCode::Down);
    assert_eq!(dialog(&app).zone, 1);
    assert_eq!(
        dialog(&app).queued,
        Some(Request::Records(ZoneId::new(ZONE_B)))
    );
}

#[test]
fn tab_moves_to_the_records_and_enter_opens_the_form_of_the_record() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    open_with(&mut app, idle_dialog());
    press(&mut app, KeyCode::Tab);
    assert_eq!(dialog(&app).focus, Focus::Records);
    press(&mut app, KeyCode::Enter);
    let View::Edit(form) = &dialog(&app).view else {
        panic!("the form");
    };
    assert_eq!(form.content, "203.0.113.5");
    assert!(matches!(form.target, Target::Existing(_)));
}

#[test]
fn a_adds_this_host_and_r_reads_the_zones_again() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    open_with(&mut app, idle_dialog());
    press(&mut app, KeyCode::Char('a'));
    assert_eq!(dialog(&app).queued, Some(Request::Host));
    let mut idle = idle_dialog();
    idle.settle();
    open_with(&mut app, idle);
    press(&mut app, KeyCode::Char('R'));
    assert_eq!(dialog(&app).queued, Some(Request::Zones));
}

#[test]
fn nothing_moves_and_nothing_is_asked_while_a_request_is_in_flight() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    let mut panel = idle_dialog();
    panel.enqueue(Request::Zones);
    open_with(&mut app, panel);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('r'));
    assert_eq!(dialog(&app).zone, 0);
    assert_eq!(dialog(&app).queued, Some(Request::Zones));
}

#[test]
fn typing_edits_the_focused_field_and_space_switches_the_proxy() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    let mut panel = idle_dialog();
    panel.view = View::Edit(Box::new(RecordForm::edit(
        panel.zones[0].clone(),
        panel.records[0].clone(),
    )));
    open_with(&mut app, panel);
    press(&mut app, KeyCode::Char('9'));
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char('7'));
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char(' '));
    let View::Edit(form) = &dialog(&app).view else {
        panic!("the form");
    };
    assert_eq!(form.content, "203.0.113.59");
    assert_eq!(form.ttl, "3007");
    assert_eq!(form.proxy, Proxy::On);
    assert_eq!(form.field, FormField::Proxy);
}

#[test]
fn enter_asks_for_the_confirmation_and_escape_returns_to_the_form_with_its_values() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    let mut panel = idle_dialog();
    panel.view = View::Edit(Box::new(RecordForm::edit(
        panel.zones[0].clone(),
        panel.records[0].clone(),
    )));
    open_with(&mut app, panel);
    press(&mut app, KeyCode::Char('1'));
    press(&mut app, KeyCode::Enter);
    assert_eq!(dialog(&app).view.kind(), ViewKind::Confirm);
    press(&mut app, KeyCode::Esc);
    let View::Edit(form) = &dialog(&app).view else {
        panic!("the form");
    };
    assert_eq!(form.content, "203.0.113.51");
}

#[test]
fn enter_on_the_confirmation_queues_the_write_and_escape_does_not_during_a_save() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    let mut panel = idle_dialog();
    let form = RecordForm::edit(panel.zones[0].clone(), panel.records[0].clone());
    panel.view = View::Edit(Box::new(form));
    open_with(&mut app, panel);
    press(&mut app, KeyCode::Char('1'));
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    assert!(matches!(dialog(&app).queued, Some(Request::Write(_))));
    press(&mut app, KeyCode::Esc);
    assert_eq!(
        dialog(&app).view.kind(),
        ViewKind::Confirm,
        "a save is not abandoned"
    );
}

#[cfg(test)]
mod keys;
