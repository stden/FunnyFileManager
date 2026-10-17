//! Purpose: write a file's new contents through the existing file — keeping
//! its inode, owner, mode and other names — with every refusal that can be
//! known in advance met before the first old byte is gone.

use std::{fs, io, path::Path};

use super::copy_file::write_contents;
use super::keep_times::{file_times, Times};
use super::{CopyRules, FileMode};

/// Replace the contents of the existing regular file `dst` with `input`'s,
/// writing through it the way `cp` does.
///
/// Nothing a rename would change changes: the inode, so every other name of
/// the file sees the new contents; the owner; and the mode, unless the rules
/// restate the source's. The times follow the rules.
///
/// The old contents go only once everything that can be refused has been:
/// `dst` must open for writing, and a time or mode the rules ask for is first
/// tried out by restating what the file already has, since the kernel lets
/// only the owner set either. What is left is a write that fails half-way —
/// a full disk, an I/O error — or a process killed mid-write, which leave the
/// file cut short: the price of keeping its identity, the one `cp`, and this
/// program's own editor for a file it may not replace, pay too.
pub fn rewrite_file(input: &fs::File, dst: &Path, rules: CopyRules) -> io::Result<()> {
    let mut output = open_for_writing(dst)?;
    let existing = output.metadata()?;
    if !existing.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    if rules.times == Times::Source {
        output.set_times(file_times(&existing)?)?;
    }
    if rules.modes == FileMode::Source {
        output.set_permissions(existing.permissions())?;
    }
    output.set_len(0)?;
    write_contents(input, &mut output, rules)
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
