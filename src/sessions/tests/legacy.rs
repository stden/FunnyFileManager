//! Purpose: Stores that earlier builds wrote still load, and the first save
//! after one writes the current form. A tag that names no origin stays refused.

use std::path::PathBuf;

use super::{fs, session, SessionStore, TempDir};
use crate::models::Origin;

/// `ffm.yml` as the builds that wrote origins as YAML tags left it: a manual
/// origin as `Manual`, an import as `!NetBoxFile` or as `!Things`.
const TAGGED_YAML: &str = "\
sessions:
- name: u@manual.example.test
  protocol: Sftp
  host: manual.example.test
  port: 22
  user: u
  auth: Password
  origin: Manual
- name: u@nb.example.test
  protocol: Sftp
  host: nb.example.test
  port: 22
  user: u
  auth: Password
  origin: !NetBoxFile /work/my exports/Démo.netbox
- name: u@things.example.test
  protocol: Sftp
  host: things.example.test
  port: 22
  user: u
  auth: Password
  origin: !Things
    machine: '7: ü #1'
";

/// `sessions.json`, the store before `ffm.yml`: origins in serde's default
/// encoding, a manual one as the string `Manual`.
const LEGACY_JSON: &str = r#"{
  "sessions": [
    {"name": "u@manual.example.test", "protocol": "Sftp", "host": "manual.example.test",
     "port": 22, "user": "u", "auth": "Password", "origin": "Manual"},
    {"name": "u@nb.example.test", "protocol": "Sftp", "host": "nb.example.test",
     "port": 22, "user": "u", "auth": "Password",
     "origin": {"NetBoxFile": "/work/my exports/Démo.netbox"}},
    {"name": "u@things.example.test", "protocol": "Sftp", "host": "things.example.test",
     "port": 22, "user": "u", "auth": "Password",
     "origin": {"Things": {"machine": "7: ü #1"}}}
  ]
}"#;

fn netbox() -> Origin {
    Origin::NetBoxFile {
        path: PathBuf::from("/work/my exports/Démo.netbox"),
    }
}

fn things() -> Origin {
    Origin::Things {
        machine: "7: ü #1".to_owned(),
    }
}

/// The origins of `store`, in the panel's order.
fn origins(store: &SessionStore) -> Vec<Origin> {
    store
        .sorted()
        .into_iter()
        .map(|session| session.origin.clone())
        .collect()
}

#[test]
fn ffm_yml_with_tagged_origins_from_earlier_builds_loads() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("ffm.yml");
    fs::write(&path, TAGGED_YAML).expect("ffm.yml");
    let store = SessionStore::load_from(path);
    assert!(!store.is_damaged(), "the tagged file must load");
    assert_eq!(origins(&store), [Origin::Manual, netbox(), things()]);
}

#[test]
fn the_first_save_after_an_earlier_build_writes_no_tag() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("ffm.yml");
    fs::write(&path, TAGGED_YAML).expect("ffm.yml");
    let mut store = SessionStore::load_from(path.clone());
    store.upsert(session("later.example.test"));
    store.save().expect("the save is not refused");
    let text = fs::read_to_string(&path).expect("ffm.yml");
    assert!(!text.contains('!'), "{text}");
    let reloaded = SessionStore::load_from(path);
    assert!(!reloaded.is_damaged());
    assert_eq!(
        origins(&reloaded),
        [Origin::Manual, Origin::Manual, netbox(), things()]
    );
}

#[test]
fn sessions_json_from_before_ffm_yml_loads_and_its_first_save_writes_ffm_yml() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("sessions.json"), LEGACY_JSON).expect("sessions.json");
    let path = dir.path().join("ffm.yml");
    let mut store = SessionStore::load_from(path.clone());
    assert!(!store.is_damaged(), "the JSON store must load");
    assert_eq!(origins(&store), [Origin::Manual, netbox(), things()]);
    store.upsert(session("later.example.test"));
    store.save().expect("the first save is not refused");
    let text = fs::read_to_string(&path).expect("ffm.yml");
    assert!(!text.contains('!'), "{text}");
    assert_eq!(
        origins(&SessionStore::load_from(path)),
        [Origin::Manual, Origin::Manual, netbox(), things()]
    );
}

#[test]
fn a_tag_that_is_not_a_session_origin_is_still_refused() {
    let texts = [
        "sessions:\n- name: u@x.example.test\n  protocol: Sftp\n  host: x.example.test\n  port: 22\n  origin: !Elsewhere 1\n",
        "sessions: []\nnote: !x 1\n",
        "sessions:\n- name: u@x.example.test\n  origin: !Things\n    machine: !x m\n",
    ];
    for text in texts {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("ffm.yml");
        fs::write(&path, text).expect("ffm.yml");
        assert!(SessionStore::load_from(path).is_damaged(), "{text}");
    }
}

const MIGRATION_SOURCE: &str = "\
outside: !Things keep
sessions:
- origin: !NetBoxFile /work/export.netbox
- origin: !Things {machine: machine-7}
- origin: !Elsewhere keep
- origin: Manual
- other: !Things keep
- 7
";

const MIGRATION_EXPECTED: &str = "\
outside: !Things keep
sessions:
- origin: {NetBoxFile: /work/export.netbox}
- origin: {Things: {machine: machine-7}}
- origin: !Elsewhere keep
- origin: Manual
- other: !Things keep
- 7
";

#[test]
fn untag_legacy_origins_preserves_unrelated_tags_and_is_idempotent() {
    use crate::models::session::untag_legacy_origins;

    let mut document: serde_norway::Mapping = serde_norway::from_str(MIGRATION_SOURCE).unwrap();
    let expected: serde_norway::Mapping = serde_norway::from_str(MIGRATION_EXPECTED).unwrap();
    untag_legacy_origins(&mut document);
    assert_eq!(document, expected);
    untag_legacy_origins(&mut document);
    assert_eq!(document, expected);
}
