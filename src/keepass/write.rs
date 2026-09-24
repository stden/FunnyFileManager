//! Purpose: entry create, upsert, update and delete with tracked edits and atomic saves.

use std::path::Path;

use keepass::Database;

use super::{build_key, open_db, save_atomic, EntryDetails, EntryId, KeepassError};

mod fields;
mod update;

pub(super) use fields::write_entry_fields;
pub use update::{update_entry, update_entry_by_id};

/// Create a brand-new kdbx4 database at `db_path` containing `entries` under the
/// root group. Refuses to overwrite an existing file ([`KeepassError::AlreadyExists`]).
pub fn create_database(
    db_path: &Path,
    password: &str,
    keyfile: Option<&Path>,
    entries: &[EntryDetails],
) -> Result<(), KeepassError> {
    if db_path.exists() {
        return Err(KeepassError::AlreadyExists);
    }
    let mut db = Database::new();
    {
        let mut root = db.root_mut();
        for details in entries {
            let mut entry = root.add_entry();
            write_entry_fields(&mut entry, details);
        }
    }
    let key = build_key(password, keyfile)?;
    save_atomic(&db, db_path, key)
}

/// Insert-or-update the entry titled `entry.title` in the root group.
///
/// Opens `db_path`, upserts (case-insensitive title match), and saves back. The
/// write path is experimental, so a `<db>.bak` backup is made and the save is atomic.
pub fn upsert_entry(
    db_path: &Path,
    password: &str,
    keyfile: Option<&Path>,
    entry: &EntryDetails,
) -> Result<(), KeepassError> {
    let mut db = open_db(db_path, password, keyfile)?;
    let existing = db.root().entry_by_name(&entry.title).map(|e| e.id());
    {
        let mut root = db.root_mut();
        match existing {
            // `EntryMut`'s setters touch fields only. The update MUST go through
            // `edit_tracking` (`EntryTrack`), which stamps `times.last_modification`
            // and pushes the previous state into the entry's history: a KeePass
            // merge/sync rejects an edit whose modification time did not advance
            // (`EntryModificationTimeNotUpdated`) and silently keeps the peer's
            // older value, and without history there is nothing to recover from.
            Some(id) => {
                root.entry_mut(id)
                    .ok_or(KeepassError::EntryNotFound)?
                    .edit_tracking(|tracked| write_entry_fields(tracked, entry));
            }
            // A fresh entry already carries `Times::new()` and has no prior state
            // worth recording, so the plain path is enough.
            None => write_entry_fields(&mut root.add_entry(), entry),
        }
    }
    let key = build_key(password, keyfile)?;
    save_atomic(&db, db_path, key)
}

/// Delete an entry by identity and save the database atomically.
pub fn delete_entry_by_id(
    db_path: &Path,
    password: &str,
    keyfile: Option<&Path>,
    id: EntryId,
) -> Result<(), KeepassError> {
    let mut db = open_db(db_path, password, keyfile)?;
    db.entry_mut(id)
        .ok_or(KeepassError::EntryNotFound)?
        .track_changes()
        .remove();
    let key = build_key(password, keyfile)?;
    save_atomic(&db, db_path, key)
}
