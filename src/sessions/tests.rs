//! Unit tests: the store keeps one session per name, sorts for display,
//! adopts imports without overwriting the user's own entries, round-trips
//! through an owner-only file, and never writes over a file it could not
//! read.

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

use super::{SessionStore, FILE_NAME, MAX_FILE_BYTES};
use crate::models::{HostName, Origin, Port, Protocol, Session, UserName};

mod file;
mod read_text;
mod safety;
mod size;
mod store;

fn session(host: &str) -> Session {
    Session::new(
        Protocol::Sftp,
        HostName::new(host).expect("host"),
        Port::SSH,
        Some(UserName::new("u").expect("user")),
    )
}

fn imported(host: &str) -> Session {
    Session {
        origin: Origin::NetBoxFile(PathBuf::from("/srv/x/demo.netbox")),
        ..session(host)
    }
}

fn file_store(dir: &TempDir) -> SessionStore {
    SessionStore::load_from(dir.path().join(FILE_NAME))
}

mod yaml;
