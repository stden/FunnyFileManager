//! Purpose: One bounded, regular-file-only read shared by the F3 viewer, the
//! F4 editor, and every configuration and data file FFM keeps — settings,
//! user menu, associations, histories, bookmarks, sessions — and the one
//! open that never waits on a FIFO ([`open_regular`]).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::Path;

use crate::utils::MAX_FILE_SIZE;

mod open;

pub use open::open_regular;

/// The largest configuration or data file [`read_small_file`] reads.
pub const MAX_SMALL_FILE_BYTES: u64 = 1_048_576;

/// What reading a file for a text UI produced.
pub enum Loaded {
    /// The file's bytes, never more than the limit asked for.
    Bytes(Vec<u8>),
    /// A directory, FIFO, socket, or device node — nothing a line-oriented UI
    /// can show, and several of them are actively dangerous to open.
    NotRegular,
    /// Over the limit; carries the size to report, in bytes.
    TooLarge(u64),
}

/// Read `path` for a text UI, refusing what such a UI cannot survive.
///
/// SECURITY / LIVENESS GUARD — both halves are load-bearing, do not "simplify"
/// either one away:
///
/// * The `is_file` check is what stops FIFOs and device nodes before opening.
/// * The bounded `take` is what stops a file that lies about (or changes) its
///   size between the `metadata` call and the read.
pub fn read_for_text_ui(path: &Path) -> io::Result<Loaded> {
    read_regular(path, MAX_FILE_SIZE.bytes())
}

/// [`read_for_text_ui`] with the limit given: at most `limit` bytes of a
/// regular file, under the same two guards.
pub fn read_regular(path: &Path, limit: u64) -> io::Result<Loaded> {
    read_regular_after_stat(path, limit, || {})
}

fn read_regular_after_stat(
    path: &Path,
    limit: u64,
    after_stat: impl FnOnce(),
) -> io::Result<Loaded> {
    // Follows symlinks on purpose: a link to a regular file is viewable, and
    // it is the *target*'s type that decides whether opening it is safe.
    let meta = fs::metadata(path)?;
    if !meta.is_file() {
        return Ok(Loaded::NotRegular);
    }
    if meta.len() > limit {
        return Ok(Loaded::TooLarge(meta.len()));
    }
    after_stat();

    let bytes = read_bounded(open_regular(path, OpenOptions::new().read(true))?, limit)?;
    let read_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if read_len > limit {
        return Ok(Loaded::TooLarge(read_len));
    }
    Ok(Loaded::Bytes(bytes))
}

/// The text of a small configuration or data file, with `what` naming it in
/// every error: at most [`MAX_SMALL_FILE_BYTES`] of UTF-8 in a regular file.
pub fn read_small_file(path: &Path, what: &str) -> io::Result<String> {
    let loaded = read_regular(path, MAX_SMALL_FILE_BYTES)
        .map_err(|error| io::Error::new(error.kind(), format!("{what}: {error}")))?;
    match loaded {
        Loaded::Bytes(bytes) => String::from_utf8(bytes).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{what}: not UTF-8 text ({error})"),
            )
        }),
        Loaded::NotRegular => Err(io::Error::other(format!("{what} is not a regular file"))),
        Loaded::TooLarge(_) => Err(io::Error::other(format!("{what} exceeds 1 MiB"))),
    }
}

/// Read at most one byte past the limit from `file`, pre-sizing from the open
/// handle's bounded length while still relying on `take` to cap endless streams.
fn read_bounded(file: File, limit: u64) -> io::Result<Vec<u8>> {
    let hint = file.metadata().map_or(0, |meta| meta.len().min(limit));
    let mut bytes = Vec::with_capacity(usize::try_from(hint).unwrap_or(0));
    file.take(limit.saturating_add(1)).read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests;
