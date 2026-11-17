//! Purpose: `NetBox` `.netbox` import and legacy `!NetBoxFile` / `!Things` YAML tag migration.

use std::fs;

use cucumber::{given, then, when};

use super::{fixture, BehaviorWorld};
use crate::command::SessionsCommand;

#[given(expr = "a NetBox export file {string} with session {string} on {string}")]
fn write_netbox_export(world: &mut BehaviorWorld, file: String, name: String, host: String) {
    let xml = format!(
        "<NetBox><Sessions>\
         <Session name=\"{name}\"><HostName>{host}</HostName><Password>0A0B</Password></Session>\
         </Sessions></NetBox>"
    );
    let path = world.case().root.path().join("left").join(file);
    fs::write(path, xml).unwrap();
    world.case().app.left_panel.load_entries().unwrap();
}

#[when(expr = "I import the NetBox export {string}")]
fn import_netbox_export(world: &mut BehaviorWorld, file: String) {
    let export_path = world.case().root.path().join("left").join(file);
    let case = world.case();
    crate::ops::sessions::execute(
        &mut case.app,
        SessionsCommand::ImportNetBox(Some(export_path)),
    )
    .unwrap();
    case.paint();
}

#[given("the private profile configuration holds legacy tagged origins")]
fn write_legacy_tagged_origins(world: &mut BehaviorWorld) {
    let yaml = "sessions:
  - name: tagged-netbox
    protocol: Sftp
    host: nb.example.test
    port: 22
    auth: Password
    origin: !NetBoxFile /tmp/exported.netbox
  - name: tagged-things
    protocol: Sftp
    host: th.example.test
    port: 22
    auth: Password
    origin: !Things
      machine: srv-01
";
    fs::write(&fixture(world).path, yaml).unwrap();
}

#[then(expr = "server profile {string} is saved with tag-free origin kind {string}")]
fn verify_tag_free_origin_kind(world: &mut BehaviorWorld, profile: String, kind: String) {
    let raw = fs::read_to_string(&fixture(world).path).unwrap();
    assert!(!raw.contains("!NetBoxFile"), "unexpected tag in:\n{raw}");
    assert!(!raw.contains("!Things"), "unexpected tag in:\n{raw}");
    let document = fixture(world).document();
    let entry = document["sessions"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|item| item["name"].as_str() == Some(profile.as_str()))
        .unwrap_or_else(|| panic!("missing session {profile:?} in:\n{raw}"));
    assert_eq!(entry["origin"]["kind"].as_str(), Some(kind.as_str()));
}

#[then("the saved configuration YAML contains no legacy origin tags")]
fn verify_no_legacy_tags(world: &mut BehaviorWorld) {
    let raw = fs::read_to_string(&fixture(world).path).unwrap();
    assert!(
        !raw.contains("!NetBoxFile"),
        "unexpected !NetBoxFile:\n{raw}"
    );
    assert!(!raw.contains("!Things"), "unexpected !Things:\n{raw}");
}
