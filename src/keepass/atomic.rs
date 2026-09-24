//! Purpose: Corruption-safe saving — the owner-only `<db>.bak` backup, a
//! co-located temp file renamed over the vault — and restoring that backup.

use std::path::{Path, PathBuf};

use keepass::{Database, DatabaseKey};

use super::KeepassError;

/// Restore the single backup created by the latest successful KDBX write.
///
/// Refused, like a save, when this process may not write the vault; the
/// restored vault keeps the vault's permissions, not the backup's 0600.
pub fn restore_backup(db_path: &Path) -> Result<(), KeepassError> {
    let backup = PathBuf::from(format!("{}.bak", db_path.display()));
    if !backup.exists() {
        return Err(KeepassError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no KDBX change to undo",
        )));
    }
    crate::file_write::ensure_writable(db_path)?;
    let bytes = std::fs::read(backup)?;
    let temp = crate::file_write::temp_beside(db_path)?;
    crate::file_write::persist_over(temp, db_path, &bytes)?;
    Ok(())
}

/// Save `db` over `db_path` corruption-safely: back up an existing target to
/// `<db>.bak`, write to a co-located temp file, fsync, then atomically rename.
/// The temp file shares the target's directory so the rename stays atomic
/// (a cross-filesystem rename would not be).
///
/// The rename checks only the directory, so a vault this process may not
/// write (0444, or another user's) is refused before anything is touched,
/// and the temp file takes the vault's permissions so a save never changes
/// them. The database is serialized first, so a serialization failure leaves
/// the vault and its backup as they were.
pub(super) fn save_atomic(
    db: &Database,
    db_path: &Path,
    key: DatabaseKey,
) -> Result<(), KeepassError> {
    crate::file_write::ensure_writable(db_path)?;
    let mut bytes = Vec::new();
    db.save(&mut bytes, key)
        .map_err(|e| KeepassError::Save(e.to_string()))?;
    if db_path.exists() {
        let backup = PathBuf::from(format!("{}.bak", db_path.display()));
        write_private_backup(db_path, &backup)?;
    }
    let temp = crate::file_write::temp_beside(db_path)?;
    crate::file_write::persist_over(temp, db_path, &bytes)?;
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
