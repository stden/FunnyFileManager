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

mod anchor;

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
    for ch in source.chars() {
        if ch.is_ascii() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.extend(
                ch.to_lowercase()
                    .map(|letter| if letter == 'ς' { 'σ' } else { letter }),
            );
        }
    }
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
    let finder = memchr::memmem::Finder::new(needle.as_bytes());
    let fast_anchor = anchor::safe_anchor(needle);
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
        if memchr::memchr(0, bytes).is_some() {
            return Ok(Scan::Absent);
        }
        if skip_without_anchor(fast_anchor, needle.len(), bytes, &mut undecoded, &mut text) {
            if read == 0 {
                return Ok(Scan::Absent);
            }
            continue;
        }
        push_chunk(read, bytes, &mut undecoded, &mut text);
        if finder.find(text.as_bytes()).is_some() {
            return Ok(Scan::Found);
        }
        if read == 0 {
            return Ok(Scan::Absent);
        }
        anchor::keep_tail(&mut text, needle.len());
    }
}

fn skip_without_anchor(
    fast_anchor: Option<u8>,
    needle_len: usize,
    bytes: &[u8],
    undecoded: &mut Vec<u8>,
    text: &mut String,
) -> bool {
    if !anchor::ruled_out(fast_anchor, text, bytes) {
        return false;
    }
    if bytes.is_empty() {
        return true;
    }
    let Some(cut) = anchor::tail_cut(bytes, needle_len) else {
        return false;
    };
    undecoded.clear();
    text.clear();
    anchor::decode_slice(bytes.get(cut..).unwrap_or_default(), false, undecoded, text);
    anchor::keep_tail(text, needle_len);
    true
}

fn push_chunk(read: usize, bytes: &[u8], undecoded: &mut Vec<u8>, text: &mut String) {
    if read == 0 {
        if !undecoded.is_empty() {
            let pending = std::mem::take(undecoded);
            anchor::decode_slice(&pending, true, undecoded, text);
        }
        return;
    }
    let skip = if undecoded.is_empty() {
        0
    } else {
        stitch_boundary(bytes, undecoded, text)
    };
    let rest = bytes.get(skip..).unwrap_or_default();
    anchor::decode_slice(rest, false, undecoded, text);
}

fn stitch_boundary(bytes: &[u8], undecoded: &mut Vec<u8>, text: &mut String) -> usize {
    let prev = undecoded.len();
    let take = bytes.len().min(4_usize.saturating_sub(prev));
    undecoded.extend_from_slice(bytes.get(..take).unwrap_or_default());
    decode_complete(undecoded, text);
    if bytes.len() <= take {
        return bytes.len();
    }
    let consumed = (prev + take).saturating_sub(undecoded.len());
    undecoded.clear();
    consumed.saturating_sub(prev)
}

/// Decode `bytes` as `String::from_utf8_lossy` would, except that a sequence
/// cut short at the very end stays in `bytes`: the next chunk completes it.
fn decode_complete(bytes: &mut Vec<u8>, out: &mut String) {
    let source = std::mem::take(bytes);
    anchor::decode_slice(&source, false, bytes, out);
}

#[cfg(test)]
mod tests;
