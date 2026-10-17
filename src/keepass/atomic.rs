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
    let target = crate::file_write::real_target(db_path)
        .map_err(|link| KeepassError::Io(link.into_io_error()))?;
    let backup = PathBuf::from(format!("{}.bak", target.display()));
    if !backup.exists() {
        return Err(KeepassError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no KDBX change to undo",
        )));
    }
    crate::file_write::ensure_writable(&target)?;
    let bytes = std::fs::read(backup)?;
    let temp = crate::file_write::temp_beside(&target)?;
    crate::file_write::persist_over(temp, &target, &bytes)?;
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
    let target = crate::file_write::real_target(db_path)
        .map_err(|link| KeepassError::Io(link.into_io_error()))?;
    crate::file_write::ensure_writable(&target)?;
    let mut bytes = Vec::new();
    db.save(&mut bytes, key)
        .map_err(|e| KeepassError::Save(e.to_string()))?;
    if target.exists() {
        let backup = PathBuf::from(format!("{}.bak", target.display()));
        write_private_backup(&target, &backup)?;
    }
    let temp = crate::file_write::temp_beside(&target)?;
    crate::file_write::persist_over(temp, &target, &bytes)?;
    Ok(())
}

/// Copy `src` onto `dst`, keeping the copy owner-only on unix.
///
/// `fs::copy` propagates the source mode, so a 0644 vault would leave a
/// world-readable `<db>.bak` sitting next to it — and the backup is kept on
/// purpose (the writer is upstream-experimental), so it must not be the weak
/// link. `replace_owner_only` writes via an owner-only co-located temp file
/// and refuses hard links or files owned by another user.
fn write_private_backup(src: &Path, dst: &Path) -> std::io::Result<()> {
    let bytes = std::fs::read(src)?;
    crate::file_write::replace_owner_only_bytes(dst, "backup", &bytes)
}
