//! Purpose: entry create, upsert, update and delete with tracked edits and atomic saves.

use std::path::Path;

use keepass::db::{fields, EntryMut, EntryTrack, GroupRef};
use keepass::Database;

use super::{build_key, open_db, save_atomic, EntryDetails, EntryId, KeepassError};

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

/// Update the entry titled `original_title` in the group at `group_path`,
/// overwriting its five standard fields with `entry`.
///
/// Resolution is [`super::read_entry`]'s: `&[]` = root, matching case-insensitive. A
/// changed `entry.title` renames the entry in place — unlike [`upsert_entry`],
/// which is root-only by name and would add a duplicate sibling instead.
/// Renaming onto a title already present in the group is refused
/// ([`KeepassError::AlreadyExists`]): the by-name read path takes the first
/// match, so a silent duplicate would permanently shadow one of the two
/// entries. The write path is experimental, so a `<db>.bak` backup is made and
/// the save is atomic ([`save_atomic`]).
pub fn update_entry(
    db_path: &Path,
    password: &str,
    keyfile: Option<&Path>,
    group_path: &[&str],
    original_title: &str,
    entry: &EntryDetails,
) -> Result<(), KeepassError> {
    let mut db = open_db(db_path, password, keyfile)?;
    let id = {
        let root = db.root();
        let group = root
            .group_by_path(group_path)
            .ok_or(KeepassError::EntryNotFound)?;
        let id = group
            .entry_by_name(original_title)
            .map(|e| e.id())
            .ok_or(KeepassError::EntryNotFound)?;
        refuse_rename_collision(&group, original_title, &entry.title)?;
        id
    };
    write_tracked_and_save(&mut db, db_path, password, keyfile, id, entry)
}

/// [`update_entry`] for an entry known by id.
///
/// This is the call the vault browser makes, so editing the second of two
/// same-titled entries writes to that entry and not to the first one a name
/// lookup would find.
///
/// `group_path` is still the entry's group: the rename-collision check needs
/// its siblings, and an id that is not in that group is treated as missing.
pub fn update_entry_by_id(
    db_path: &Path,
    password: &str,
    keyfile: Option<&Path>,
    group_path: &[&str],
    id: EntryId,
    entry: &EntryDetails,
) -> Result<(), KeepassError> {
    let mut db = open_db(db_path, password, keyfile)?;
    {
        let root = db.root();
        let group = root
            .group_by_path(group_path)
            .ok_or(KeepassError::EntryNotFound)?;
        let current = group.entry(id).ok_or(KeepassError::EntryNotFound)?;
        let original_title = current.get(fields::TITLE).unwrap_or_default().to_string();
        refuse_rename_collision(&group, &original_title, &entry.title)?;
    }
    write_tracked_and_save(&mut db, db_path, password, keyfile, id, entry)
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

/// Refuse a rename onto a title the group already has.
///
/// Titles are compared the way `entry_by_name` matches them, so only a real
/// rename is checked — re-saving under the same title (any case) must not
/// trip over the entry itself.
fn refuse_rename_collision(
    group: &GroupRef<'_>,
    original_title: &str,
    new_title: &str,
) -> Result<(), KeepassError> {
    let renames = !new_title.eq_ignore_ascii_case(original_title);
    if renames && group.entry_by_name(new_title).is_some() {
        return Err(KeepassError::AlreadyExists);
    }
    Ok(())
}

fn write_tracked_and_save(
    db: &mut Database,
    db_path: &Path,
    password: &str,
    keyfile: Option<&Path>,
    id: EntryId,
    entry: &EntryDetails,
) -> Result<(), KeepassError> {
    // `Database::entry_mut` resolves the id database-wide. The root group's
    // `entry_mut` sees only its direct children and would miss the nested
    // entry that `group_by_path` just resolved. Same `edit_tracking`
    // requirement as `upsert_entry`: it stamps `last_modification` and records
    // history, or a KeePass sync rejects the edit and keeps the peer's value.
    db.entry_mut(id)
        .ok_or(KeepassError::EntryNotFound)?
        .edit_tracking(|tracked| write_entry_fields(tracked, entry));
    let key = build_key(password, keyfile)?;
    save_atomic(db, db_path, key)
}

/// The entry setters shared by [`EntryMut`] and [`EntryTrack`], so the list of
/// standard fields in [`write_entry_fields`] lives in exactly one place.
pub(super) trait EntryFieldWriter {
    fn set_unprotected_field(&mut self, key: &str, value: &str);
    fn set_protected_field(&mut self, key: &str, value: &str);
}

impl EntryFieldWriter for EntryMut<'_> {
    fn set_unprotected_field(&mut self, key: &str, value: &str) {
        self.set_unprotected(key, value);
    }

    fn set_protected_field(&mut self, key: &str, value: &str) {
        self.set_protected(key, value);
    }
}

impl EntryFieldWriter for EntryTrack<'_> {
    fn set_unprotected_field(&mut self, key: &str, value: &str) {
        self.set_unprotected(key, value);
    }

    fn set_protected_field(&mut self, key: &str, value: &str) {
        self.set_protected(key, value);
    }
}

/// Write the five standard fields onto an entry (password is stored protected).
pub(super) fn write_entry_fields(entry: &mut impl EntryFieldWriter, details: &EntryDetails) {
    entry.set_unprotected_field(fields::TITLE, details.title.as_str());
    entry.set_unprotected_field(fields::USERNAME, details.username.as_str());
    entry.set_unprotected_field(fields::URL, details.url.as_str());
    entry.set_unprotected_field(fields::NOTES, details.notes.as_str());
    entry.set_protected_field(fields::PASSWORD, details.password.as_str());
}
