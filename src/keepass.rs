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

use keepass::db::DatabaseOpenError;
/// An entry's identity, re-exported so the vault browser can address an
/// entry without going through its title.
pub use keepass::db::EntryId;
#[cfg(test)]
use keepass::db::{fields, Times};
use keepass::{Database, DatabaseKey};

mod export;
mod read;
mod write;

#[cfg(test)]
use export::export_times;
pub use export::{
    export, export_json, export_yaml, to_json, to_yaml, ExportEntry, ExportGroup, ExportTimes,
    KdbxExport,
};
pub(crate) use read::read_catalog;
pub use read::{list_entries, read_entry, read_entry_by_id};
#[cfg(test)]
use write::write_entry_fields;
pub use write::{
    create_database, delete_entry_by_id, update_entry, update_entry_by_id, upsert_entry,
};

/// A flat, secret-free summary of one entry, for listing in the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntrySummary {
    /// The entry's own identity. Titles are not unique in `KeePass` — two
    /// `GitHub` rows in one group are legal, and common after a rotation — so
    /// every read and write of a listed entry goes by this, never by name:
    /// resolving by name takes the first match, which for the second row is
    /// the wrong entry.
    pub id: EntryId,
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

impl From<DatabaseOpenError> for KeepassError {
    fn from(error: DatabaseOpenError) -> Self {
        match error {
            // A read may fail after File::open succeeds. Preserve its category
            // so an inaccessible database is not presented as a bad password.
            DatabaseOpenError::Io(error) => Self::Io(error),
            other => Self::Open(other.to_string()),
        }
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
        key = key.with_keyfile(&mut file)?;
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
    Database::open(&mut file, key).map_err(KeepassError::from)
}

/// Restore the single backup created by the latest successful KDBX write.
pub(crate) fn restore_backup(db_path: &Path) -> Result<(), KeepassError> {
    let backup = PathBuf::from(format!("{}.bak", db_path.display()));
    if !backup.exists() {
        return Err(KeepassError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no KDBX change to undo",
        )));
    }
    let dir = db_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    std::io::copy(&mut std::fs::File::open(backup)?, tmp.as_file_mut())?;
    tmp.as_file().sync_all()?;
    tmp.persist(db_path)
        .map_err(|error| KeepassError::Io(error.error))?;
    Ok(())
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

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_external_vault;

#[cfg(test)]
mod tests_read_errors;
