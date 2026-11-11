//! Purpose: Create inert executable fixtures without depending on installed host programs.

use std::{fs, io, path::Path};

/// Writes `contents` to `path` and makes the file executable.
pub fn write_executable(path: &Path, contents: &[u8]) -> io::Result<()> {
    write_contents(path, contents)?;
    permissions(path)
}

/// Unix writes the file from a child shell, not from this process. A sibling
/// test thread that forks while this process holds `path` open hands its child
/// a copy of the descriptor, and running `path` then fails with ETXTBSY ("text
/// file busy") until that child execs. The shell closes its descriptor before
/// it exits, so no copy outlives this call.
#[cfg(unix)]
fn write_contents(path: &Path, contents: &[u8]) -> io::Result<()> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::process::{Command, Stdio};
    let status = Command::new("/bin/sh")
        .args(["-c", r#"printf '%s' "$2" > "$1""#, "sh"])
        .arg(path)
        .arg(OsStr::from_bytes(contents))
        .stdin(Stdio::null())
        .status()?;
    if status.success() {
        return Ok(());
    }
    Err(io::Error::other(format!(
        "could not write {}",
        path.display()
    )))
}

#[cfg(windows)]
fn write_contents(path: &Path, contents: &[u8]) -> io::Result<()> {
    fs::write(path, contents)
}

#[cfg(unix)]
fn permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
}

#[cfg(windows)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "shares the Unix signature, where the operation can fail"
)]
fn permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(all(test, unix))]
mod tests;
