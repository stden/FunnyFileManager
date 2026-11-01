//! Purpose: BDD setup and actions for private network mount snapshots.
use super::BehaviorWorld;
use crate::models::{
    HostName, MountPoint, NetworkAddress, NetworkDrive, NetworkProtocol, Port, Protocol, Session,
};
use crate::sessions::SessionStore;
use cucumber::{given, when};
use std::{fs, io, path::PathBuf};

mod checks;

pub(super) struct Fixture {
    discovered: Vec<NetworkDrive>,
    outcome: Outcome,
    original: Vec<u8>,
}
enum Outcome {
    NotSaved,
    Saved,
    Rejected(io::ErrorKind),
}

fn path(world: &mut BehaviorWorld) -> PathBuf {
    world.case().root.path().join("ffm.yml")
}

#[given("a private configuration with a saved server and custom settings")]
fn configuration(world: &mut BehaviorWorld) {
    let path = path(world);
    let mut store = SessionStore::load_from(path.clone());
    store.upsert(Session::new(
        Protocol::Sftp,
        HostName::new("server.example.test").unwrap(),
        Port::SSH,
        None,
    ));
    store.save().unwrap();
    let text = fs::read_to_string(&path).unwrap();
    fs::write(&path, format!("{text}\ncustom:\n  theme: blue\n")).unwrap();
    world.mounts = Some(Fixture {
        discovered: Vec::new(),
        outcome: Outcome::NotSaved,
        original: fs::read(path).unwrap(),
    });
}

fn mapping(path: &str, remote: &str, protocol: NetworkProtocol) -> NetworkDrive {
    NetworkDrive {
        path: MountPoint::try_from(path.to_owned()).unwrap(),
        remote: NetworkAddress::try_from(remote.to_owned()).unwrap(),
        protocol,
    }
}

#[given("SMB and NFS mappings have been discovered")]
fn discovered(world: &mut BehaviorWorld) {
    world.mounts.as_mut().unwrap().discovered = vec![
        mapping("Z:\\", "//files.example.test/team", NetworkProtocol::Smb),
        mapping(
            "/mnt/team",
            "server.example.test:/export/team",
            NetworkProtocol::Nfs,
        ),
    ];
}

#[given("network mappings were previously saved")]
fn previously_saved(world: &mut BehaviorWorld) {
    discovered(world);
    save(world);
}

#[given("the private network configuration has invalid YAML")]
fn damaged(world: &mut BehaviorWorld) {
    let path = path(world);
    let original = b"network_drives: [unfinished\n";
    fs::write(path, original).unwrap();
    world.mounts.as_mut().unwrap().original = original.to_vec();
}

#[when("I save the detected network mappings")]
fn save(world: &mut BehaviorWorld) {
    let path = path(world);
    let fixture = world.mounts.as_mut().unwrap();
    fixture.original = fs::read(&path).unwrap();
    fixture.outcome = match crate::ops::network_drives::save_to(&path, &fixture.discovered) {
        Ok(()) => Outcome::Saved,
        Err(error) => Outcome::Rejected(error.kind()),
    };
}

#[when("discovery returns no network mappings")]
fn offline(world: &mut BehaviorWorld) {
    world.mounts.as_mut().unwrap().discovered.clear();
    save(world);
}

#[when(expr = "drive Z is rediscovered at {string}")]
fn rediscovered(world: &mut BehaviorWorld, remote: String) {
    world.mounts.as_mut().unwrap().discovered =
        vec![mapping("Z:\\", &remote, NetworkProtocol::Smb)];
    save(world);
}

fn document(world: &mut BehaviorWorld) -> serde_norway::Value {
    serde_norway::from_slice(&fs::read(path(world)).unwrap()).unwrap()
}
