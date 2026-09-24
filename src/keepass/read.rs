//! Purpose: secret-free entry listing and full entry reads of an opened vault.

use std::path::Path;

use keepass::db::{fields, EntryRef, GroupRef};

use super::{open_db, EntryDetails, EntryId, EntrySummary, KeepassError};

/// List every entry in the database as a flat, secret-free [`EntrySummary`] list.
/// Passwords are never read by this call.
pub fn list_entries(
    db_path: &Path,
    password: &str,
    keyfile: Option<&Path>,
) -> Result<Vec<EntrySummary>, KeepassError> {
    let db = open_db(db_path, password, keyfile)?;
    let mut out = Vec::new();
    collect_summaries(&db.root(), &mut Vec::new(), &mut out);
    Ok(out)
}

pub struct Catalog {
    pub entries: Vec<EntrySummary>,
    pub groups: Vec<Vec<String>>,
}

pub fn read_catalog(
    path: &Path,
    password: &str,
    keyfile: Option<&Path>,
) -> Result<Catalog, KeepassError> {
    let db = open_db(path, password, keyfile)?;
    let mut catalog = Catalog {
        entries: Vec::new(),
        groups: Vec::new(),
    };
    collect_summaries(&db.root(), &mut Vec::new(), &mut catalog.entries);
    collect_groups(&db.root(), &mut Vec::new(), &mut catalog.groups);
    Ok(catalog)
}

fn collect_groups(group: &GroupRef<'_>, path: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    out.push(path.clone());
    for sub in group.groups() {
        path.push(sub.name.clone());
        collect_groups(&sub, path, out);
        path.pop();
    }
}

/// Depth-first walk that records each entry with its group path relative to root.
/// `path` is the walk's current position; it is restored before returning.
fn collect_summaries(group: &GroupRef<'_>, path: &mut Vec<String>, out: &mut Vec<EntrySummary>) {
    for entry in group.entries() {
        out.push(EntrySummary {
            id: entry.id(),
            title: entry.get(fields::TITLE).unwrap_or_default().to_owned(),
            username: entry.get(fields::USERNAME).unwrap_or_default().to_owned(),
            url: entry.get(fields::URL).unwrap_or_default().to_owned(),
            group_path: path.clone(),
        });
    }
    for sub in group.groups() {
        path.push(sub.name.clone());
        collect_summaries(&sub, path, out);
        path.pop();
    }
}

/// Read full details (incl. the plaintext password) for the entry titled `title`
/// in the group at `group_path` (`&[]` = root, `&["Work", "CI"]` = nested).
/// Matching is case-insensitive.
///
/// The caller owns the returned password and should drop it promptly.
pub fn read_entry(
    db_path: &Path,
    password: &str,
    keyfile: Option<&Path>,
    group_path: &[&str],
    title: &str,
) -> Result<EntryDetails, KeepassError> {
    let db = open_db(db_path, password, keyfile)?;
    let root = db.root();
    // group_by_path(&[]) resolves to the root group itself.
    let group = root
        .group_by_path(group_path)
        .ok_or(KeepassError::EntryNotFound)?;
    let entry = group
        .entry_by_name(title)
        .ok_or(KeepassError::EntryNotFound)?;
    Ok(details_of(&entry))
}

/// Read full details (incl. the plaintext password) of the entry `id`,
/// wherever it sits in the tree.
///
/// This is the lookup the vault browser uses: a summary carries the id it was
/// listed with, so a duplicate title cannot hand the user another entry's
/// password. [`read_entry`] stays for callers that only know a name.
pub fn read_entry_by_id(
    db_path: &Path,
    password: &str,
    keyfile: Option<&Path>,
    id: EntryId,
) -> Result<EntryDetails, KeepassError> {
    let db = open_db(db_path, password, keyfile)?;
    let entry = db.entry(id).ok_or(KeepassError::EntryNotFound)?;
    Ok(details_of(&entry))
}

fn details_of(entry: &EntryRef<'_>) -> EntryDetails {
    EntryDetails {
        title: entry.get(fields::TITLE).unwrap_or_default().to_owned(),
        username: entry.get(fields::USERNAME).unwrap_or_default().to_owned(),
        url: entry.get(fields::URL).unwrap_or_default().to_owned(),
        notes: entry.get(fields::NOTES).unwrap_or_default().to_owned(),
        password: entry.get_password().unwrap_or_default().to_owned(),
    }
}
