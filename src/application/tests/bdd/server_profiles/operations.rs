//! Purpose: `NetBox` dialog keys persist profiles and real reloads observe the user's YAML.

use super::{fixture, name, reload, session, BehaviorWorld, Case, SessionStore, WriteAttempt};
use crate::models::{Overlay, PanelKind};
use cucumber::when;
use std::fs;

#[when("I open the user server profile panel")]
#[when("I reload the user server profiles")]
fn open_profiles(world: &mut BehaviorWorld) {
    reload(world);
}

#[when(expr = "I create saved server profile {string}")]
fn create_profile(world: &mut BehaviorWorld, host: String) {
    create(world, &host);
    assert!(matches!(fixture(world).attempt, WriteAttempt::Saved));
}

#[when(expr = "I attempt to create saved server profile {string}")]
fn attempt_profile(world: &mut BehaviorWorld, host: String) {
    create(world, &host);
}

fn create(world: &mut BehaviorWorld, host: &str) {
    let previous = fs::read(&fixture(world).path).unwrap_or_default();
    let case = world.case();
    case.press("Shift+F4 Tab Tab");
    case.type_text(host);
    case.press("Enter");
    let attempt = if matches!(case.app.overlay, Overlay::SessionEdit(_)) {
        let status = case.app.status_text();
        assert!(
            status.contains("did not load") || status.contains("Not saved:"),
            "{status}"
        );
        WriteAttempt::Refused { previous }
    } else {
        assert!(!case.app.overlay.is_active());
        assert!(case.app.sessions.find(&name(host)).is_some());
        WriteAttempt::Saved
    };
    fixture(world).attempt = attempt;
}

#[when(expr = "I change server profile {string} to port {int}")]
fn edit_port(world: &mut BehaviorWorld, profile: String, port: u16) {
    let case = world.case();
    let name = name(&profile);
    let digits = case
        .app
        .sessions
        .find(&name)
        .unwrap()
        .port
        .to_string()
        .len();
    focus(case, &name);
    case.press("F4 Tab Tab Tab");
    case.press(&"Backspace ".repeat(digits));
    case.type_text(&port.to_string());
    case.press("Enter");
    assert!(!case.app.overlay.is_active());
}

#[when(expr = "I delete saved server profile {string}")]
fn delete_profile(world: &mut BehaviorWorld, profile: String) {
    let case = world.case();
    focus(case, &name(&profile));
    case.press("F8 y");
    assert!(!case.app.overlay.is_active());
}

fn focus(case: &mut Case, name: &crate::models::SessionName) {
    let PanelKind::Sessions(panel) = &mut case.app.get_active_panel().kind else {
        panic!("the user profiles must be open");
    };
    panel.focus(name);
    assert_eq!(panel.selected_name(), Some(name));
}

#[when(expr = "another session saves server profile {string}")]
fn concurrent_profile(world: &mut BehaviorWorld, host: String) {
    let mut store = SessionStore::load_from(fixture(world).path.clone());
    store.upsert(session(&host));
    store.save().unwrap();
}
