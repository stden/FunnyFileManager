//! Purpose: Judge an archive's member listing before anything is unpacked:
//! refuse a path that leaves the destination and anything but plain files
//! and folders.

use std::io;

/// Refuse an archive that would write outside its destination or leave a
/// link behind: a member named with `..`, an absolute path, a drive, or any
/// type other than a plain file or directory.
pub(super) fn validate_listing(names: &str, details: &str) -> io::Result<()> {
    if names.lines().any(leaves_destination) {
        return Err(io::Error::other(
            "Archive contains a path outside its destination",
        ));
    }
    if details
        .lines()
        .filter_map(member_type)
        .any(|kind| !matches!(kind, '-' | 'd' | '?'))
    {
        return Err(io::Error::other(
            "Extraction supports regular files and directories only; links and special files are refused",
        ));
    }
    Ok(())
}

/// Whether a listed member name reaches outside the destination: it is
/// absolute, or one of its steps is `..`.
///
/// Both `/` and `\\` count as separators, because `unzip` turns a DOS-made
/// archive's `\\` into `/`. That is also why a backslash is not refused on
/// its own: on Unix it is an ordinary character, and `tar -tf` prints a tab,
/// a backslash or (under `LANG=C`) any non-ASCII byte as a backslash escape,
/// which never hides a `/` or a `.`.
fn leaves_destination(name: &str) -> bool {
    name.starts_with(['/', '\\'])
        || name.split(['/', '\\']).any(|step| step == "..")
        || names_a_drive_or_stream(name)
}

/// A colon on Windows starts a drive (`C:x`) or an alternate data stream
/// (`file:stream`); on Unix it is an ordinary character (`meeting 10:30`).
#[cfg(windows)]
fn names_a_drive_or_stream(name: &str) -> bool {
    name.contains(':')
}

#[cfg(not(windows))]
const fn names_a_drive_or_stream(_name: &str) -> bool {
    false
}

/// The type letter of a member line — the first of its ten-character mode.
///
/// `tar -tvf` prints nothing but member lines; `unzip -Z` wraps its rows in
/// a header and a footer, which have no mode and so answer `None`. A `?` is a
/// zip member stored without type bits, which `unzip` writes as a file.
pub(super) fn member_type(line: &str) -> Option<char> {
    let mode = line.get(..10)?;
    let shaped = line.as_bytes().get(10).is_none_or(|byte| *byte == b' ')
        && mode.chars().all(|ch| "-dlhbcps?rwxsStT".contains(ch));
    shaped.then(|| mode.chars().next()).flatten()
}
