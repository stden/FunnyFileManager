//! Purpose: write a file's new contents through the existing file — keeping
//! its inode, owner, mode and other names — with every refusal that can be
//! known in advance met before the first old byte is gone.

use std::{fs, io, path::Path};

use super::copy_file::write_contents;
use super::keep_times::{file_times, Times};
use super::{CopyRules, FileMode};

/// Replace the contents of the existing regular file `dst` with `input`'s,
/// writing through it the way `cp` does.
pub fn rewrite_file(input: &fs::File, dst: &Path, rules: CopyRules) -> io::Result<()> {
    let mut output = open_for_writing(dst)?;
    let existing = output.metadata()?;
    ensure_regular_file(existing.is_file())?;
    ensure_distinct_open_file(input, &existing)?;
    if rules.times == Times::Source {
        output.set_times(file_times(&existing)?)?;
    }
    if rules.modes == FileMode::Source {
        output.set_permissions(existing.permissions())?;
    }
    output.set_len(0)?;
    write_contents(input, &mut output, rules)
}

fn ensure_regular_file(is_file: bool) -> io::Result<()> {
    if is_file {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        ))
    }
}

#[cfg(unix)]
fn ensure_distinct_open_file(input: &fs::File, existing: &fs::Metadata) -> io::Result<()> {
    use std::os::unix::fs::MetadataExt;
    if input
        .metadata()
        .is_ok_and(|src| src.dev() == existing.dev() && src.ino() == existing.ino())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot copy or move onto itself",
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "matches the Unix self-overwrite check signature"
)]
const fn ensure_distinct_open_file(_input: &fs::File, _existing: &fs::Metadata) -> io::Result<()> {
    Ok(())
}

/// Open `dst` for writing, never waiting: something swapped in for the file
/// since it was listed — a FIFO — would otherwise hold the event loop in
/// `open` until a reader came. A regular file opens as ever.
#[cfg(unix)]
fn open_for_writing(dst: &Path) -> io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;

    fs::OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(dst)
}

#[cfg(not(unix))]
fn open_for_writing(dst: &Path) -> io::Result<fs::File> {
    fs::OpenOptions::new().write(true).open(dst)
}

#[cfg(test)]
mod tests;
