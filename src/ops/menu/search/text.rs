//! Purpose: Whether a file's text contains the needle, read a chunk at a time
//! so a file of any size is searched in bounded memory.
//!
//! The whole file used to be read into memory, so anything over 10 MB was
//! skipped, and skipped silently: a big log never showed up in Alt+F7's
//! results. Now only one chunk and a needle's length of the text before it
//! are held at once, and the search's own time limit, not the file's size,
//! is what can cut a scan short.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use std::time::Instant;

/// How much of a file is read at a time.
const CHUNK: usize = 64 * 1024;

/// What scanning one file found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Scan {
    Found,
    /// Not in the file, or the file is binary or could not be read.
    Absent,
    /// The search's time ran out before the file was read to its end.
    OutOfTime,
}

/// Lowercase `text` one character at a time, so the needle and every chunk
/// of a file fold the same way wherever a chunk happens to end. `str::
/// to_lowercase` looks at the neighbours of a capital sigma, which a chunk
/// boundary would hide; folding the final sigma to the plain one takes that
/// choice away altogether.
pub(super) fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    fold_into(text, &mut out);
    out
}

fn fold_into(source: &str, out: &mut String) {
    out.reserve(source.len());
    if source.is_ascii() {
        let start = out.len();
        out.push_str(source);
        if let Some(appended) = out.get_mut(start..) {
            appended.make_ascii_lowercase();
        }
        return;
    }
    out.extend(source.chars().flat_map(char::to_lowercase).map(|letter| {
        if letter == 'ς' {
            'σ'
        } else {
            letter
        }
    }));
}

/// Scan the regular file at `path` for `needle`, already [`fold`]ed.
///
/// Only a regular file is opened: a FIFO would block the open forever. It is
/// read up to the size it had when asked, so a log that keeps growing cannot
/// keep the search going. A file that cannot be read counts as not holding
/// the text, as a binary one does.
pub(super) fn file_contains(path: &Path, needle: &str, deadline: Instant) -> Scan {
    let Ok(metadata) = fs::metadata(path) else {
        return Scan::Absent;
    };
    if !metadata.is_file() {
        return Scan::Absent;
    }
    File::open(path)
        .and_then(|file| contains_text(file.take(metadata.len()), needle, deadline))
        .unwrap_or(Scan::Absent)
}

/// Whether the text `reader` yields contains `needle`, decoded as
/// `String::from_utf8_lossy` decodes it. A NUL byte marks the file binary,
/// which is not searched.
fn contains_text(mut reader: impl Read, needle: &str, deadline: Instant) -> io::Result<Scan> {
    let mut chunk = vec![0_u8; CHUNK];
    let mut undecoded = Vec::new();
    let mut text = String::new();
    loop {
        if Instant::now() >= deadline {
            return Ok(Scan::OutOfTime);
        }
        crate::progress::tick();
        let read = reader.read(&mut chunk)?;
        let bytes = chunk.get(..read).unwrap_or_default();
        if bytes.contains(&0) {
            return Ok(Scan::Absent);
        }
        push_chunk(read, bytes, &mut undecoded, &mut text);
        if text.contains(needle) {
            return Ok(Scan::Found);
        }
        if read == 0 {
            return Ok(Scan::Absent);
        }
        keep_tail(&mut text, needle.len());
    }
}

fn push_chunk(read: usize, bytes: &[u8], undecoded: &mut Vec<u8>, text: &mut String) {
    if read == 0 {
        undecoded.extend_from_slice(bytes);
        fold_into(&String::from_utf8_lossy(undecoded), text);
        return;
    }
    if undecoded.is_empty() {
        if let Ok(valid) = std::str::from_utf8(bytes) {
            fold_into(valid, text);
            return;
        }
    }
    undecoded.extend_from_slice(bytes);
    decode_complete(undecoded, text);
}

/// Decode `bytes` as `String::from_utf8_lossy` would, except that a sequence
/// cut short at the very end stays in `bytes`: the next chunk completes it.
fn decode_complete(bytes: &mut Vec<u8>, out: &mut String) {
    let mut start = 0;
    while let Some(rest) = bytes.get(start..) {
        let error = match std::str::from_utf8(rest) {
            Ok(valid) => {
                fold_into(valid, out);
                start = bytes.len();
                break;
            }
            Err(error) => error,
        };
        let valid = rest.get(..error.valid_up_to()).unwrap_or_default();
        fold_into(&String::from_utf8_lossy(valid), out);
        start += error.valid_up_to();
        let Some(invalid) = error.error_len() else {
            break;
        };
        out.push(char::REPLACEMENT_CHARACTER);
        start += invalid;
    }
    bytes.drain(..start.min(bytes.len()));
}

/// Keep only what a match that starts before the next chunk could use: the
/// last `needle_len - 1` bytes, widened to a character boundary.
fn keep_tail(text: &mut String, needle_len: usize) {
    let cut = text.floor_char_boundary(text.len().saturating_sub(needle_len.saturating_sub(1)));
    text.drain(..cut);
}

#[cfg(test)]
mod tests;
