//! Purpose: Isolated profile files, legacy migration input and unrelated YAML sections.

use super::{fixture, session, BehaviorWorld, Fixture, SessionStore};
use cucumber::given;
use std::fs;

pub(super) const OTHER_SECTIONS: &str = r"custom:
  theme: amber
network_drives:
  - path: 'Z:\'
    remote: '\\example.test\share'
    protocol: smb
";

#[given("a private ffm.yml for server profiles")]
fn private_file(world: &mut BehaviorWorld) {
    world.profiles = Some(Fixture::new(world.case()));
}

#[given(expr = "a legacy server profile {string}")]
fn legacy_profile(world: &mut BehaviorWorld, host: String) {
    let legacy = fixture(world).path.with_file_name("sessions.json");
    let mut store = SessionStore::load_from(legacy.clone());
    store.upsert(session(&host));
    store.save().unwrap();
    fixture(world).legacy = Some(fs::read(legacy).unwrap());
}

#[given("the private profile configuration contains network drives and custom options")]
fn unrelated_settings(world: &mut BehaviorWorld) {
    let path = fixture(world).path.clone();
    fs::write(path, format!("{OTHER_SECTIONS}sessions: []\n")).unwrap();
}

#[given(expr = "a {word} server profile document")]
fn unusable_document(world: &mut BehaviorWorld, damage: String) {
    let path = fixture(world).path.clone();
    let text = match damage.as_str() {
        "malformed" => "sessions: [".to_owned(),
        "duplicate" => {
            let session = serde_json::to_string(&session("duplicate.example.test")).unwrap();
            format!("{{\"sessions\":[{session},{session}]}}")
        }
        _ => panic!("unknown profile damage fixture {damage}"),
    };
    fs::write(path, text).unwrap();
}
