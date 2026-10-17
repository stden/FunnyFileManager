//! Purpose: updating one entry in place — by title or by id — with tracked
//! edits, refusing a rename onto a sibling's title.

use std::path::Path;

use keepass::db::{fields, GroupRef};
use keepass::Database;

use super::write_entry_fields;
use crate::keepass::{build_key, open_db, save_atomic, EntryDetails, EntryId, KeepassError};

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
        let original_title = current.get(fields::TITLE).unwrap_or_default().to_owned();
        refuse_rename_collision(&group, &original_title, &entry.title)?;
    }
    write_tracked_and_save(&mut db, db_path, password, keyfile, id, entry)
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
