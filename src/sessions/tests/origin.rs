//! Purpose: A session's origin survives a save and a restart, so a file that
//! holds an imported session still loads and still takes later saves.

use std::path::PathBuf;

use super::{session, SessionStore, TempDir};
use crate::models::{Origin, Session};

fn with_origin(origin: Origin) -> Session {
    Session {
        origin,
        ..session("origin.example.test")
    }
}

fn netbox(path: &str) -> Origin {
    Origin::NetBoxFile {
        path: PathBuf::from(path),
    }
}

fn things(machine: &str) -> Origin {
    Origin::Things {
        machine: machine.to_owned(),
    }
}

/// Paths and machine ids with spaces, non-ASCII text and the marks YAML reads
/// as syntax: each one reloads as the session that was saved.
#[test]
fn each_origin_survives_a_save_and_a_reload() {
    let origins = [
        Origin::Manual,
        netbox("/srv/x/demo.netbox"),
        things("machine-7"),
        netbox("/srv/my exports/Démo ✓ #1.netbox"),
        netbox("/srv/x/!tag: 'q' \"d\" &a *b.netbox"),
        things("7: ü #1 {x} [y] 'q' \"d\" !t &a *s"),
        things("null"),
        things("line\nbreak"),
    ];
    for origin in origins {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("ffm.yml");
        let entry = with_origin(origin.clone());
        let mut store = SessionStore::load_from(path.clone());
        store.upsert(entry.clone());
        store.save().expect("save");

        let reloaded = SessionStore::load_from(path);
        assert!(!reloaded.is_damaged(), "{origin:?} did not reload");
        assert_eq!(reloaded.find(&entry.name), Some(&entry), "{origin:?}");
    }
}

#[test]
fn an_imported_things_session_loads_after_a_restart_and_later_saves_work() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("ffm.yml");
    let mut store = SessionStore::load_from(path.clone());
    store.upsert(with_origin(things("machine-7")));
    store.save().expect("the import is saved");

    let mut restarted = SessionStore::load_from(path.clone());
    assert!(!restarted.is_damaged(), "the import loads after a restart");
    assert_eq!(restarted.len(), 1);
    restarted.upsert(session("later.example.test"));
    restarted
        .save()
        .expect("a save after the restart is not refused");
    assert_eq!(SessionStore::load_from(path).len(), 2);
}

#[test]
fn a_manual_origin_is_not_written_at_all() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("ffm.yml");
    let mut store = SessionStore::load_from(path.clone());
    store.upsert(session("manual.example.test"));
    store.save().expect("save");
    let text = std::fs::read_to_string(&path).expect("ffm.yml");
    assert!(!text.contains("origin"), "{text}");
}
