//! Purpose: Whether a file's text contains the needle, read a chunk at a time
//! so a file of any size is searched in bounded memory.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use std::time::Instant;

mod anchor;
pub(super) use anchor::fold;
#[cfg(test)]
use anchor::{decode_complete, fold_into};
use anchor::{push_chunk, skip_without_anchor};

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

/// Reusable buffer and precomputed needle metadata across all files in one search.
pub(super) struct Scanner {
    needle: String,
    finder: memchr::memmem::Finder<'static>,
    fast_anchor: Option<u8>,
    ascii_safe: bool,
    chunk: Vec<u8>,
    undecoded: Vec<u8>,
    text: String,
}

impl Scanner {
    pub(super) fn new(needle: String) -> Self {
        let finder = memchr::memmem::Finder::new(needle.as_bytes()).into_owned();
        let fast_anchor = anchor::safe_anchor(&needle);
        let ascii_safe =
            !needle.is_empty() && crate::casefold::is_ascii_safe_needle(needle.as_bytes());
        Self {
            needle,
            finder,
            fast_anchor,
            ascii_safe,
            chunk: Vec::new(),
            undecoded: Vec::new(),
            text: String::new(),
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.needle.is_empty()
    }

    pub(super) fn file_contains(
        &mut self,
        path: &Path,
        kind: Option<fs::FileType>,
        deadline: Instant,
    ) -> Scan {
        if !kind.is_some_and(is_regular_file_type)
            && !fs::metadata(path).is_ok_and(|meta| meta.is_file())
        {
            return Scan::Absent;
        }
        self.open_and_scan(path, deadline)
    }

    fn open_and_scan(&mut self, path: &Path, deadline: Instant) -> Scan {
        File::open(path)
            .and_then(|file| {
                let len = file.metadata()?.len();
                self.scan_reader(file.take(len), deadline)
            })
            .unwrap_or(Scan::Absent)
    }

    fn reset_buffers(&mut self) -> usize {
        let max_tail = if self.ascii_safe {
            self.needle.len().saturating_sub(1)
        } else {
            0
        };
        self.chunk.resize(max_tail + CHUNK, 0);
        self.undecoded.clear();
        self.text.clear();
        max_tail
    }

    fn scan_reader(&mut self, mut reader: impl Read, deadline: Instant) -> io::Result<Scan> {
        let max_tail = self.reset_buffers();
        let mut tail_len = 0;
        loop {
            if Instant::now() >= deadline {
                return Ok(Scan::OutOfTime);
            }
            crate::progress::tick();
            let buf = self
                .chunk
                .get_mut(tail_len..tail_len + CHUNK)
                .unwrap_or_default();
            let read = reader.read(buf)?;
            let fresh = self
                .chunk
                .get(tail_len..tail_len + read)
                .unwrap_or_default();
            if memchr::memchr(0, fresh).is_some() {
                return Ok(Scan::Absent);
            }
            let outcome = if self.ascii_safe {
                self.step_ascii(read, &mut tail_len, max_tail)
            } else {
                self.step_unicode(read)
            };
            if let Some(done) = outcome {
                return Ok(done);
            }
        }
    }

    fn step_ascii(&mut self, read: usize, tail_len: &mut usize, max_tail: usize) -> Option<Scan> {
        if read == 0 {
            return Some(Scan::Absent);
        }
        let end = *tail_len + read;
        let window = self.chunk.get(..end).unwrap_or_default();
        if crate::casefold::find_ascii_ignore_case(window, self.needle.as_bytes()).is_some() {
            return Some(Scan::Found);
        }
        let keep = max_tail.min(end);
        self.chunk.copy_within(end - keep..end, 0);
        *tail_len = keep;
        None
    }

    fn step_unicode(&mut self, read: usize) -> Option<Scan> {
        let bytes = self.chunk.get(..read).unwrap_or_default();
        if skip_without_anchor(
            self.fast_anchor,
            self.needle.len(),
            bytes,
            &mut self.undecoded,
            &mut self.text,
        ) {
            if read == 0 {
                return Some(Scan::Absent);
            }
            return None;
        }
        push_chunk(read, bytes, &mut self.undecoded, &mut self.text);
        if self.finder.find(self.text.as_bytes()).is_some() {
            return Some(Scan::Found);
        }
        if read == 0 {
            return Some(Scan::Absent);
        }
        anchor::keep_tail(&mut self.text, self.needle.len());
        None
    }
}

#[cfg(unix)]
fn is_regular_file_type(ft: fs::FileType) -> bool {
    use std::os::unix::fs::FileTypeExt;
    let special = ft.is_fifo() || ft.is_socket() || ft.is_char_device() || ft.is_block_device();
    !ft.is_dir() && !ft.is_symlink() && !special
}

#[cfg(not(unix))]
fn is_regular_file_type(ft: fs::FileType) -> bool {
    !ft.is_dir() && !ft.is_symlink()
}

#[cfg(test)]
pub(super) fn file_contains(path: &Path, needle: &str, deadline: Instant) -> Scan {
    Scanner::new(needle.to_owned()).file_contains(path, None, deadline)
}

#[cfg(test)]
fn contains_text(reader: impl Read, needle: &str, deadline: Instant) -> io::Result<Scan> {
    Scanner::new(needle.to_owned()).scan_reader(reader, deadline)
}

#[cfg(test)]
mod tests;
