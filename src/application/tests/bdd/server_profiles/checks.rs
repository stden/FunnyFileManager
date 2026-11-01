//! Purpose: Assertions observe saved profile contents, rendered state and failure preservation.

use super::{fixture, name, setup::OTHER_SECTIONS, BehaviorWorld, SessionStore, WriteAttempt};
use crate::models::Overlay;
use cucumber::then;
use std::fs;

#[then(expr = "server profile {string} is saved with port {int}")]
fn stored_port(world: &mut BehaviorWorld, profile: String, port: u16) {
    let path = fixture(world).path.clone();
    let store = SessionStore::load_from(path);
    assert_eq!(store.find(&name(&profile)).unwrap().port.get(), port);
    assert_eq!(
        world
            .case()
            .app
            .sessions
            .find(&name(&profile))
            .unwrap()
            .port
            .get(),
        port
    );
}

#[then(expr = "server profile {string} is absent after reload")]
fn absent_profile(world: &mut BehaviorWorld, profile: String) {
    let store = SessionStore::load_from(fixture(world).path.clone());
    assert!(store.find(&name(&profile)).is_none());
    assert!(world.case().app.sessions.find(&name(&profile)).is_none());
}

#[then("the original legacy server profile file is unchanged")]
fn legacy_unchanged(world: &mut BehaviorWorld) {
    let fixture = fixture(world);
    assert_eq!(
        fs::read(fixture.path.with_file_name("sessions.json")).unwrap(),
        *fixture.legacy.as_ref().unwrap(),
    );
}

#[then("network drive mappings and custom profile options remain unchanged")]
fn unrelated_unchanged(world: &mut BehaviorWorld) {
    let document = fixture(world).document();
    let before: serde_norway::Value = serde_norway::from_str(OTHER_SECTIONS).unwrap();
    for key in ["custom", "network_drives"] {
        assert_eq!(document[key], before[key]);
    }
}

#[then("saved profiles describe authentication without credential fields")]
fn no_credentials(world: &mut BehaviorWorld) {
    let document = fixture(world).document();
    let sessions = document["sessions"].as_sequence().unwrap();
    assert_ne!(sessions.len(), 0);
    for session in sessions {
        let fields = session.as_mapping().unwrap();
        assert_eq!(session["auth"].as_str(), Some("Password"));
        for forbidden in ["password", "passphrase", "token", "secret"] {
            assert!(!fields.contains_key(serde_norway::Value::String(forbidden.to_owned())));
        }
    }
}

#[then("the rejected profile save preserves the existing configuration bytes")]
fn refused_save(world: &mut BehaviorWorld) {
    let fixture = fixture(world);
    let WriteAttempt::Refused { previous } = &fixture.attempt else {
        panic!("the profile save must have failed");
    };
    assert_eq!(fs::read(&fixture.path).unwrap(), *previous);
}

#[then("the profile dialog remains open for correction")]
fn correction_dialog(world: &mut BehaviorWorld) {
    assert!(matches!(world.case().app.overlay, Overlay::SessionEdit(_)));
}
