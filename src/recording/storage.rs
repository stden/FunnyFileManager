//! Purpose: Where a recording goes by default, and the private directory it goes in.

use std::io;
use std::path::{Path, PathBuf};

/// Where `--record` puts a session when it is given no path.
pub(super) fn default_path() -> io::Result<PathBuf> {
    let root = default_root();
    create_private_dir(&root)?;
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    Ok(root.join(format!("session-{time}-{}.py", std::process::id())))
}

/// The temporary directory is shared by every local user and the first one to
/// create a shared name owns it, so each user gets a directory of their own.
pub(super) fn default_root() -> PathBuf {
    #[cfg(unix)]
    let name = format!("ffm-recordings-{}", crate::platform::real_uid());
    #[cfg(not(unix))]
    let name = String::from("ffm-recordings");
    std::env::temp_dir().join(name)
}

/// Create the recordings directory as 0700 rather than at the mercy of the
/// umask: a recording names the paths its author browsed and holds the keys
/// they typed.
pub(super) fn create_private_dir(path: &Path) -> io::Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    ensure_private(path)
}

/// `create_dir_all` is happy with a directory that was already there, whatever
/// its owner and mode — and the temporary directory is world-writable, so a
/// hostile local user can put one there first under this user's name. Ours
/// gets tightened; anyone else's is refused, which disables the recording
/// rather than writing keystrokes somewhere someone else chose.
#[cfg(unix)]
fn ensure_private(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    // `symlink_metadata`, so a symlink pointing at somewhere agreeable cannot
    // answer for the directory it stands in front of.
    let info = std::fs::symlink_metadata(path)?;
    let us = crate::platform::real_uid();
    if !info.is_dir() || info.uid() != us {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{}: not a directory of your own", path.display()),
        ));
    }
    if info.permissions().mode() & 0o077 != 0 {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Windows has no equivalent of the shared, world-writable temporary
/// directory this guards against: each user's is their own.
#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "shares the Unix signature, where the operation can fail"
)]
const fn ensure_private(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(all(test, unix))]
mod tests;
