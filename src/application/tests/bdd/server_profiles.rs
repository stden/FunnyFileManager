//! Purpose: Server-profile BDD state and fixtures for the private user configuration.

use super::{BehaviorWorld, Case};
use crate::models::{HostName, Port, Protocol, Session, SessionName};
use crate::sessions::SessionStore;
use std::{fs, path::PathBuf};

mod checks;
mod operations;
mod setup;

pub(super) struct Fixture {
    path: PathBuf,
    legacy: Option<Vec<u8>>,
    attempt: WriteAttempt,
}

enum WriteAttempt {
    NotAttempted,
    Saved,
    Refused { previous: Vec<u8> },
}

impl Fixture {
    fn new(case: &Case) -> Self {
        Self {
            path: case.root.path().join("ffm.yml"),
            legacy: None,
            attempt: WriteAttempt::NotAttempted,
        }
    }

    fn document(&self) -> serde_norway::Value {
        serde_norway::from_str(&fs::read_to_string(&self.path).unwrap()).unwrap()
    }
}

fn fixture(world: &mut BehaviorWorld) -> &mut Fixture {
    world
        .profiles
        .as_mut()
        .expect("the private profile configuration Given runs first")
}

fn name(text: &str) -> SessionName {
    SessionName::new(text).unwrap()
}

fn session(host: &str) -> Session {
    Session::new(
        Protocol::Sftp,
        HostName::new(host).unwrap(),
        Port::SSH,
        None,
    )
}

fn reload(world: &mut BehaviorWorld) {
    let store = SessionStore::load_from(fixture(world).path.clone());
    let case = world.case();
    case.app.close_overlay();
    crate::ops::sessions::install(&mut case.app, store);
    crate::ops::sessions::open(&mut case.app);
    case.paint();
}
