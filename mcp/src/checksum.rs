//! Purpose: Streaming SHA-256 and edge fingerprints, as the checksum,
//! duplicate and comparison tools report them.

use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::Path;

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";
const EDGE_SAMPLE_BYTES: usize = 8 * 1024;
const EDGE_SAMPLE_OFFSET: u64 = 8 * 1024;
pub const EDGE_THRESHOLD_BYTES: u64 = EDGE_SAMPLE_OFFSET * 2;

const DEFAULT_HASH_BUFFER_BYTES: usize = 64 * 1024;
const MIN_HASH_BUFFER_BYTES: usize = 8 * 1024;
const MAX_HASH_BUFFER_BYTES: usize = 256 * 1024;

/// A file's SHA-256, or `None` if it cannot be read.
pub fn checksum_of(path: &Path) -> Option<String> {
    checksum_file(path).ok()
}

/// SHA-256 of `path`, streaming so a large duplicate costs a read, not a copy.
pub fn checksum_file(path: &Path) -> io::Result<String> {
    let file = fs::File::open(path)?;
    let cap = hash_buffer_capacity(&file);
    finish_reader_hash(io::BufReader::with_capacity(cap, file), Sha256::new())
}

fn hash_buffer_capacity(file: &fs::File) -> usize {
    file.metadata().map_or(DEFAULT_HASH_BUFFER_BYTES, |meta| {
        usize::try_from(meta.len())
            .unwrap_or(MAX_HASH_BUFFER_BYTES)
            .clamp(MIN_HASH_BUFFER_BYTES, MAX_HASH_BUFFER_BYTES)
    })
}

/// Drain `reader` into `hasher` and format the lowercase hex digest.
pub fn finish_reader_hash<R: io::Read>(
    mut reader: io::BufReader<R>,
    mut hasher: Sha256,
) -> io::Result<String> {
    use io::BufRead;

    loop {
        let block = reader.fill_buf()?;
        if block.is_empty() {
            return Ok(hex_digest(hasher));
        }
        hasher.update(block);
        let length = block.len();
        reader.consume(length);
    }
}

/// Format a finalized [`Sha256`] state as a 64-char lowercase hex string.
pub fn hex_digest(hasher: Sha256) -> String {
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for &byte in &digest {
        hex.push(hex_digit(byte >> 4));
        hex.push(hex_digit(byte & 0x0f));
    }
    hex
}

#[inline]
fn hex_digit(nibble: u8) -> char {
    let ascii = HEX_DIGITS.get(usize::from(nibble)).copied().unwrap_or(b'0');
    char::from(ascii)
}

/// Fast 8 KiB head + 8 KiB tail SHA-256 pre-filter for large equal-size files.
pub fn edge_fingerprint(path: &Path, size: u64) -> Option<[u8; 32]> {
    edge_fingerprint_io(path, size).ok()
}

fn edge_fingerprint_io(path: &Path, size: u64) -> io::Result<[u8; 32]> {
    use io::{Read, Seek, SeekFrom};

    let mut file = fs::File::open(path)?;
    let mut buf = [0_u8; EDGE_SAMPLE_BYTES];
    let mut hasher = Sha256::new();
    file.read_exact(&mut buf)?;
    hasher.update(buf);
    file.seek(SeekFrom::Start(size.saturating_sub(EDGE_SAMPLE_OFFSET)))?;
    file.read_exact(&mut buf)?;
    hasher.update(buf);
    Ok(hasher.finalize().into())
}
