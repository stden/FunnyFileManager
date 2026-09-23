//! Purpose: One bounded, regular-file-only read shared by the F3 viewer and the F4 editor.
//!
//! Both entry points need the exact same three answers before they may load a
//! file into a text UI — "is this even a file?", "is it small enough?", "give
//! me its bytes" — so the rule lives here once instead of being re-derived at
//! each call site.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use crate::utils::MAX_FILE_SIZE;

/// What reading a file for a text UI produced.
///
/// One enum for both callers, so the viewer and the editor cannot disagree
/// about which cases exist; they differ only in how each case reads in the
/// status bar.
pub enum Loaded {
    /// The file's bytes, never more than [`MAX_FILE_SIZE`] of them.
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
/// * The `is_file` check is what stops FIFOs and device nodes. A size check
///   alone cannot: **every** non-regular file reports `len() == 0`, so
///   `/dev/zero` sails past it and then grows a `Vec` until the OOM killer
///   fires, and opening a writer-less FIFO blocks inside `File::open` forever
///   with the terminal still in raw mode — the event loop is wedged and Ctrl+C
///   is not even delivered as a signal. The check must happen *before* the
///   open, because the open is the thing that blocks.
/// * The bounded `take` is what stops a file that lies about (or changes) its
///   size between the `metadata` call and the read. `metadata` is a snapshot;
///   the read is where the memory is actually spent.
///
/// The size check still cannot be dropped in favour of `take` alone, either:
/// `take` would silently present the first 10 MB of a 4 GB log as if that were
/// the whole file.
pub fn read_for_text_ui(path: &Path) -> io::Result<Loaded> {
    // Follows symlinks on purpose: a link to a regular file is viewable, and
    // it is the *target*'s type that decides whether opening it is safe.
    let meta = fs::metadata(path)?;
    if !meta.is_file() {
        return Ok(Loaded::NotRegular);
    }
    if meta.len() > MAX_FILE_SIZE.bytes() {
        return Ok(Loaded::TooLarge(meta.len()));
    }

    let bytes = read_bounded(File::open(path)?)?;
    let read_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if read_len > MAX_FILE_SIZE.bytes() {
        return Ok(Loaded::TooLarge(read_len));
    }
    Ok(Loaded::Bytes(bytes))
}

/// Read at most one byte past the limit from `file`.
///
/// Split out so the bound can be tested against a genuinely endless file: the
/// `take` is the only thing that ever stops such a read. `fs::read` merely
/// *pre-sizes* its buffer from `metadata().len()` and then reads to EOF, which
/// for `/dev/zero` never comes.
///
/// The extra byte is what keeps "exactly at the limit" loadable while still
/// making "over the limit" detectable without reading all of it.
fn read_bounded(file: File) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.take(MAX_FILE_SIZE.bytes() + 1)
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests;
