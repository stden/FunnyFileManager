//! Purpose: Persisted YAML assertions for network mount acceptance scenarios.
use super::{document, path, BehaviorWorld, Outcome};
use crate::models::{NetworkDrive, NetworkProtocol};
use cucumber::then;

#[then("the network mapping update succeeds")]
fn saved(world: &mut BehaviorWorld) {
    assert!(matches!(
        world.mounts.as_ref().unwrap().outcome,
        Outcome::Saved
    ));
}

#[then("the network mapping update is refused")]
fn refused(world: &mut BehaviorWorld) {
    assert!(matches!(
        world.mounts.as_ref().unwrap().outcome,
        Outcome::Rejected(std::io::ErrorKind::InvalidData)
    ));
}

#[then(expr = "the private configuration contains {int} network mappings")]
fn count(world: &mut BehaviorWorld, expected: usize) {
    let drives: Vec<NetworkDrive> =
        serde_norway::from_value(document(world)["network_drives"].clone()).unwrap();
    assert_eq!(drives.len(), expected);
    let names: std::collections::BTreeSet<_> = drives.iter().map(|drive| &drive.path).collect();
    assert_eq!(names.len(), drives.len(), "each mount point is saved once");
}

#[then(expr = "drive Z points to {string}")]
fn target(world: &mut BehaviorWorld, expected: String) {
    let drives: Vec<NetworkDrive> =
        serde_norway::from_value(document(world)["network_drives"].clone()).unwrap();
    let drive = drives
        .iter()
        .find(|drive| String::from(drive.path.clone()) == "Z:\\")
        .unwrap();
    assert_eq!(drive.protocol, NetworkProtocol::Smb);
    assert_eq!(String::from(drive.remote.clone()), expected);
}

#[then("the saved server profiles and custom settings are unchanged")]
fn other_sections(world: &mut BehaviorWorld) {
    let saved = document(world);
    let original: serde_norway::Value =
        serde_norway::from_slice(&world.mounts.as_ref().unwrap().original).unwrap();
    assert_eq!(saved["sessions"], original["sessions"]);
    assert_eq!(saved["custom"], original["custom"]);
}

#[then("the private configuration is unchanged byte for byte")]
fn unchanged(world: &mut BehaviorWorld) {
    let saved = std::fs::read(path(world)).unwrap();
    assert_eq!(saved, world.mounts.as_ref().unwrap().original);
}
