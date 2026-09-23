//! Purpose: Download pinned HTTPS archives and reject links, traversal, and oversized extraction.

use super::{check_cancel, environment::Environment, storage, InstallError};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    process::Command,
    sync::atomic::AtomicBool,
};

pub(super) fn download(
    url: &str,
    checksum: &str,
    root: &Path,
    stop: &AtomicBool,
    environment: &dyn Environment,
) -> Result<(), InstallError> {
    let parsed = url::Url::parse(url).map_err(crate::lsp::logged(InstallError::Recipe))?;
    if parsed.scheme() != "https"
        || checksum.len() != 64
        || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(InstallError::Recipe);
    }
    let curl = bounded_curl(environment, stop)?;
    let archive = root.join("server.tar.gz");
    environment.run(
        Command::new(curl)
            .args(CURL_ARGS)
            .arg(&archive)
            .arg(url)
            .current_dir(root),
        stop,
    )?;
    verify(&archive, checksum, stop)?;
    unpack(File::open(&archive)?, root, stop)?;
    fs::remove_file(archive)?;
    Ok(())
}

fn bounded_curl(
    environment: &dyn Environment,
    stop: &AtomicBool,
) -> Result<std::path::PathBuf, InstallError> {
    const MISSING: &str = "LSP installation requires curl 8.4+ for bounded HTTPS downloads";
    let curl = environment.program(&["curl"], MISSING)?;
    let output = environment.run(Command::new(&curl).arg("--version"), stop)?;
    let text = String::from_utf8_lossy(&output);
    let version = text
        .split_whitespace()
        .nth(1)
        .ok_or(InstallError::Missing(MISSING))?;
    let mut parts = version.split('.');
    let major = parts.next().and_then(|part| part.parse::<u32>().ok());
    let minor = parts.next().and_then(|part| part.parse::<u32>().ok());
    if major.zip(minor).is_some_and(|version| version >= (8, 4)) {
        Ok(curl)
    } else {
        Err(InstallError::Missing(MISSING))
    }
}

const CURL_ARGS: &[&str] = &[
    "--fail",
    "--location",
    "--silent",
    "--show-error",
    "--proto",
    "=https",
    "--proto-redir",
    "=https",
    "--connect-timeout",
    "15",
    "--max-time",
    "300",
    "--max-filesize",
    "536870912",
    "--output",
];

fn verify(path: &Path, expected: &str, stop: &AtomicBool) -> Result<(), InstallError> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        check_cancel(stop)?;
        let length = file.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        // `read` never reports more than the buffer holds.
        hash.update(buffer.get(..length).ok_or(InstallError::Integrity)?);
    }
    if storage::hex(&hash.finalize()).eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(InstallError::Integrity)
    }
}

fn unpack(input: impl Read, root: &Path, stop: &AtomicBool) -> Result<(), InstallError> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(input));
    let mut total = 0u64;
    for (index, entry) in archive.entries()?.enumerate() {
        check_cancel(stop)?;
        let mut entry = entry?;
        total = total.saturating_add(entry.size());
        if index >= 25_000 || total > 2_147_483_648 {
            return Err(InstallError::Archive);
        }
        let path = entry.path()?.to_string_lossy().into_owned();
        let destination =
            storage::relative(root, &path).map_err(crate::lsp::logged(InstallError::Archive))?;
        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(destination)?;
        } else if entry.header().entry_type().is_file() {
            extract_file(&mut entry, &destination, stop)?;
        } else {
            return Err(InstallError::Archive);
        }
    }
    Ok(())
}

fn extract_file<R: Read>(
    entry: &mut tar::Entry<'_, R>,
    destination: &Path,
    stop: &AtomicBool,
) -> Result<(), InstallError> {
    fs::create_dir_all(destination.parent().ok_or(InstallError::Archive)?)?;
    let mut file = File::create(destination)?;
    let mut buffer = [0; 8192];
    loop {
        check_cancel(stop)?;
        let length = entry.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        // `read` never reports more than the buffer holds.
        file.write_all(buffer.get(..length).ok_or(InstallError::Archive)?)?;
    }
    permissions(&file, entry.header().mode()?)?;
    Ok(())
}

#[cfg(unix)]
fn permissions(file: &File, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(fs::Permissions::from_mode(mode & 0o777))
}

#[cfg(windows)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "shares the Unix signature, where the operation can fail"
)]
fn permissions(_file: &File, _mode: u32) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests;
