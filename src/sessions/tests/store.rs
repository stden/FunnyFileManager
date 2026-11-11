//! Purpose: The store in memory — one session per name, sorted for display,
//! and an import adopted without overwriting the user's own entries.

use super::{imported, session, SessionStore};
use crate::models::{Port, SessionName};
use crate::sessions::import::{Imported, SkipReason, Skipped};

fn name(text: &str) -> SessionName {
    SessionName::new(text).expect("name")
}

fn names(store: &SessionStore) -> Vec<&str> {
    store.sorted().iter().map(|s| s.name.as_str()).collect()
}

#[test]
fn upsert_keeps_one_session_per_name_and_sorted_orders_by_name() {
    let mut store = SessionStore::default();
    assert!(store.is_empty());
    store.upsert(session("z.test"));
    store.upsert(session("a.test"));
    assert!(!store.is_empty());
    let mut changed = session("z.test");
    changed.port = Port::new(2222).expect("port");
    changed.name = name("u@z.test");
    store.upsert(changed);
    assert_eq!(store.len(), 2);
    assert_eq!(names(&store), ["u@a.test", "u@z.test"]);
    assert_eq!(
        store.find(&name("u@z.test")).map(|s| s.port.get()),
        Some(2222)
    );
    assert!(store.remove(&name("u@a.test")).is_some());
    assert!(store.remove(&name("u@a.test")).is_none());
    assert_eq!(store.find(&name("u@a.test")), None);
    assert_eq!(store.len(), 1);
}

#[test]
fn adopt_keeps_the_users_entries_and_refreshes_imported_ones() {
    let mut store = SessionStore::default();
    store.upsert(session("manual.test"));
    store.upsert(imported("old.test"));
    let mut refreshed = imported("old.test");
    refreshed.port = Port::new(2200).expect("port");
    refreshed.name = name("u@old.test");

    let report = store.adopt(Imported {
        sessions: vec![refreshed, imported("manual.test"), imported("new.test")],
        skipped: vec![Skipped::new("junk", SkipReason::UnsupportedProtocol)],
        passwords_left_out: 2,
    });

    assert_eq!(report.imported, 2);
    assert_eq!(report.passwords_left_out, 2);
    assert_eq!(
        report.skipped,
        [
            Skipped::new("junk", SkipReason::UnsupportedProtocol),
            Skipped::new("u@manual.test", SkipReason::ManualEntryKept),
        ]
    );
    assert_eq!(report.to_string(), "2 imported, 2 skipped");
    assert_eq!(
        store.find(&name("u@old.test")).map(|s| s.port.get()),
        Some(2200)
    );
    assert!(store
        .find(&name("u@manual.test"))
        .is_some_and(|s| s.origin.is_manual()));
    assert_eq!(store.len(), 3);
}

#[test]
fn a_session_only_store_saves_nowhere_and_says_it_did() {
    let mut store = SessionStore::default();
    store.upsert(session("a.test"));
    assert!(store.save().is_ok());
    assert!(!store.is_damaged());
}
