//! Purpose: Create inert executable fixtures without depending on installed host programs.

use std::{fs, io, path::Path};

pub fn write_executable(path: &Path, contents: &[u8]) -> io::Result<()> {
    fs::write(path, contents)?;
    permissions(path)
}

#[cfg(unix)]
fn permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
}

#[cfg(windows)]
fn permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}
