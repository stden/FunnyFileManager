//! Feature-gated `KeePass` `.kdbx` read/edit support (enable with `--features keepass`).
//!
//! Reading (kdbx3/kdbx4, master password + optional key file) is stable. The
//! underlying `keepass` crate's **writing is upstream-experimental**: it
//! re-serializes the inner XML, so any field it does not model is dropped on
//! save. To bound that risk this module never truncates in place — [`save_atomic`]
//! backs the target up to `<db>.bak` (created 0600 on unix, never inheriting the
//! vault's mode), writes a co-located temp file, fsyncs, then atomically renames
//! over the original. Callers should still keep their own backup.
//!
//! Secrets: [`EntryDetails::password`] is plaintext. Never pass it to the action
//! or error logs (`ffm_actions.log` / `ffm_errors.log`) or the file viewer, and
//! drop it promptly — copied `String`s are not zeroized.

use std::path::{Path, PathBuf};

use keepass::db::{fields, EntryMut, EntryRef, EntryTrack, GroupRef, Times};
use keepass::{Database, DatabaseKey};
use serde::{Deserialize, Serialize};

/// A flat, secret-free summary of one entry, for listing in the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntrySummary {
    pub title: String,
    pub username: String,
    pub url: String,
    /// Group names from the root down to the entry's group, e.g. `[]` (root) or
    /// `["Work", "CI"]`. Kept structural rather than `/`-joined: `KeePass` allows
    /// `/` inside a group name, so a joined string cannot be split back reliably
    /// and would resolve to the wrong entry. Feed it straight to [`read_entry`].
    pub group_path: Vec<String>,
}

/// Full details of one entry, including the **plaintext** password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryDetails {
    pub title: String,
    pub username: String,
    pub url: String,
    pub notes: String,
    /// Plaintext password — do not log; drop promptly.
    pub password: String,
}

/// Errors from kdbx operations.
#[derive(Debug)]
pub enum KeepassError {
    /// Opening/decrypting failed (wrong password/key file, corrupt file, …).
    Open(String),
    /// Saving/encrypting failed.
    Save(String),
    /// Underlying filesystem error.
    Io(std::io::Error),
    /// No entry matched the requested group/title.
    EntryNotFound,
    /// [`create_database`] refused to overwrite an existing file.
    AlreadyExists,
    /// JSON/YAML serialization of an export failed.
    Serialize(String),
}

impl std::fmt::Display for KeepassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open(m) => write!(f, "failed to open kdbx: {m}"),
            Self::Save(m) => write!(f, "failed to save kdbx: {m}"),
            Self::Io(e) => write!(f, "kdbx I/O error: {e}"),
            Self::EntryNotFound => write!(f, "entry not found"),
            Self::AlreadyExists => write!(f, "destination already exists"),
            Self::Serialize(m) => write!(f, "failed to serialize export: {m}"),
        }
    }
}

impl std::error::Error for KeepassError {}

impl From<std::io::Error> for KeepassError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Build a [`DatabaseKey`] from a master password and an optional key file.
///
/// Both opening and saving go through here so the credentials are composed
/// identically — a mismatch silently breaks decryption.
fn build_key(password: &str, keyfile: Option<&Path>) -> Result<DatabaseKey, KeepassError> {
    let mut key = DatabaseKey::new().with_password(password);
    if let Some(path) = keyfile {
        let mut file = std::fs::File::open(path)?;
        key = key
            .with_keyfile(&mut file)
            .map_err(|e| KeepassError::Open(e.to_string()))?;
    }
    Ok(key)
}

/// Open and decrypt a database from disk (kdbx3/kdbx4 auto-detected).
fn open_db(
    db_path: &Path,
    password: &str,
    keyfile: Option<&Path>,
) -> Result<Database, KeepassError> {
    let key = build_key(password, keyfile)?;
    let mut file = std::fs::File::open(db_path)?;
    Database::open(&mut file, key).map_err(|e| KeepassError::Open(e.to_string()))
}

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

/// Depth-first walk that records each entry with its group path relative to root.
/// `path` is the walk's current position; it is restored before returning.
fn collect_summaries(group: &GroupRef<'_>, path: &mut Vec<String>, out: &mut Vec<EntrySummary>) {
    for entry in group.entries() {
        out.push(EntrySummary {
            title: entry.get(fields::TITLE).unwrap_or_default().to_string(),
            username: entry.get(fields::USERNAME).unwrap_or_default().to_string(),
            url: entry.get(fields::URL).unwrap_or_default().to_string(),
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
    Ok(EntryDetails {
        title: entry.get(fields::TITLE).unwrap_or_default().to_string(),
        username: entry.get(fields::USERNAME).unwrap_or_default().to_string(),
        url: entry.get(fields::URL).unwrap_or_default().to_string(),
        notes: entry.get(fields::NOTES).unwrap_or_default().to_string(),
        password: entry.get_password().unwrap_or_default().to_string(),
    })
}

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

/// The entry setters shared by [`EntryMut`] and [`EntryTrack`], so the list of
/// standard fields in [`write_entry_fields`] lives in exactly one place.
trait EntryFieldWriter {
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
fn write_entry_fields(entry: &mut impl EntryFieldWriter, details: &EntryDetails) {
    entry.set_unprotected_field(fields::TITLE, details.title.as_str());
    entry.set_unprotected_field(fields::USERNAME, details.username.as_str());
    entry.set_unprotected_field(fields::URL, details.url.as_str());
    entry.set_unprotected_field(fields::NOTES, details.notes.as_str());
    entry.set_protected_field(fields::PASSWORD, details.password.as_str());
}

/// Save `db` over `db_path` corruption-safely: back up an existing target to
/// `<db>.bak`, write to a co-located temp file, fsync, then atomically rename.
/// The temp file shares the target's directory so the rename stays atomic
/// (a cross-filesystem rename would not be).
fn save_atomic(db: &Database, db_path: &Path, key: DatabaseKey) -> Result<(), KeepassError> {
    let dir = db_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if db_path.exists() {
        let backup = PathBuf::from(format!("{}.bak", db_path.display()));
        write_private_backup(db_path, &backup)?;
    }
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    db.save(tmp.as_file_mut(), key)
        .map_err(|e| KeepassError::Save(e.to_string()))?;
    tmp.as_file().sync_all()?;
    tmp.persist(db_path)
        .map_err(|e| KeepassError::Io(e.error))?;
    Ok(())
}

/// Copy `src` onto `dst`, keeping the copy owner-only on unix.
///
/// `fs::copy` propagates the source mode, so a 0644 vault would leave a
/// world-readable `<db>.bak` sitting next to it — and the backup is kept on
/// purpose (the writer is upstream-experimental), so it must not be the weak
/// link. The mode is set at creation to avoid a readable window, and again
/// afterwards because `OpenOptions::mode` is ignored for an existing file.
fn write_private_backup(src: &Path, dst: &Path) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut backup = options.open(dst)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        backup.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    std::io::copy(&mut std::fs::File::open(src)?, &mut backup)?;
    backup.sync_all()
}

// ===================== Full export to JSON / YAML =====================

/// A complete, serializable snapshot of a kdbx database.
///
/// Contains the full group tree, every entry, and **every field — including
/// plaintext passwords**. The serialized output therefore holds secrets: write
/// it only to a protected location and delete it when done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdbxExport {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub database_name: Option<String>,
    pub root: ExportGroup,
}

/// One group in the exported tree, with its child groups and entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportGroup {
    pub name: String,
    pub uuid: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub groups: Vec<Self>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub entries: Vec<ExportEntry>,
}

/// One entry in the exported tree. `fields` holds every string field (standard
/// and custom) with protected values decrypted to plaintext.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportEntry {
    pub uuid: String,
    pub fields: std::collections::BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub attachments: Vec<String>,
    #[serde(skip_serializing_if = "ExportTimes::is_empty", default)]
    pub times: ExportTimes,
}

/// Entry timestamps as strings (each omitted from the output when unset).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportTimes {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub expires: Option<String>,
}

impl ExportTimes {
    const fn is_empty(&self) -> bool {
        self.created.is_none() && self.modified.is_none() && self.expires.is_none()
    }
}

/// Extract the entire database into a [`KdbxExport`] tree.
///
/// Includes every group, every entry, all string fields (standard and custom)
/// **with plaintext passwords**, tags, attachment names, and timestamps.
pub fn export(
    db_path: &Path,
    password: &str,
    keyfile: Option<&Path>,
) -> Result<KdbxExport, KeepassError> {
    let db = open_db(db_path, password, keyfile)?;
    Ok(KdbxExport {
        database_name: db.meta.database_name.clone(),
        root: export_group(&db.root()),
    })
}

fn export_group(group: &GroupRef<'_>) -> ExportGroup {
    ExportGroup {
        name: group.name.clone(),
        uuid: group.id().uuid().to_string(),
        notes: group.notes.clone().filter(|n| !n.is_empty()),
        tags: group.tags.clone(),
        groups: group.groups().map(|g| export_group(&g)).collect(),
        entries: group.entries().map(|e| export_entry(&e)).collect(),
    }
}

fn export_entry(entry: &EntryRef<'_>) -> ExportEntry {
    let mut fields = std::collections::BTreeMap::new();
    for key in entry.fields.keys() {
        if let Some(value) = entry.get(key) {
            fields.insert(key.clone(), value.to_string());
        }
    }
    ExportEntry {
        uuid: entry.id().uuid().to_string(),
        fields,
        tags: entry.tags.clone(),
        attachments: entry
            .attachments_named()
            .map(|(name, _)| name.to_string())
            .collect(),
        times: export_times(&entry.times),
    }
}

fn export_times(times: &Times) -> ExportTimes {
    ExportTimes {
        created: times.creation.map(|t| t.to_string()),
        modified: times.last_modification.map(|t| t.to_string()),
        expires: if times.expires == Some(true) {
            times.expiry.map(|t| t.to_string())
        } else {
            None
        },
    }
}

/// Serialize a [`KdbxExport`] to pretty JSON. The output contains plaintext secrets.
pub fn to_json(export: &KdbxExport) -> Result<String, KeepassError> {
    serde_json::to_string_pretty(export).map_err(|e| KeepassError::Serialize(e.to_string()))
}

/// Serialize a [`KdbxExport`] to YAML. The output contains plaintext secrets.
pub fn to_yaml(export: &KdbxExport) -> Result<String, KeepassError> {
    serde_norway::to_string(export).map_err(|e| KeepassError::Serialize(e.to_string()))
}

/// Open a database and return its full contents as pretty JSON (contains secrets).
pub fn export_json(
    db_path: &Path,
    password: &str,
    keyfile: Option<&Path>,
) -> Result<String, KeepassError> {
    to_json(&export(db_path, password, keyfile)?)
}

/// Open a database and return its full contents as YAML (contains secrets).
pub fn export_yaml(
    db_path: &Path,
    password: &str,
    keyfile: Option<&Path>,
) -> Result<String, KeepassError> {
    to_yaml(&export(db_path, password, keyfile)?)
}

#[cfg(test)]
mod tests;
