//! Purpose: Streaming SHA-256 of a file, as the checksum, duplicate and
//! comparison tools report it.

use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::Path;

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// A file's SHA-256, or `None` if it cannot be read.
pub fn checksum_of(path: &Path) -> Option<String> {
    checksum_file(path).ok()
}

/// SHA-256 of `path`, streaming so a large duplicate costs a read, not a copy.
pub fn checksum_file(path: &Path) -> io::Result<String> {
    use io::BufRead;

    let mut reader = io::BufReader::with_capacity(64 * 1024, fs::File::open(path)?);
    let mut hasher = Sha256::new();
    loop {
        let block = reader.fill_buf()?;
        if block.is_empty() {
            break;
        }
        hasher.update(block);
        let length = block.len();
        reader.consume(length);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for &byte in &digest {
        let hi = HEX_DIGITS
            .get(usize::from(byte >> 4))
            .copied()
            .unwrap_or(b'0');
        let lo = HEX_DIGITS
            .get(usize::from(byte & 0x0f))
            .copied()
            .unwrap_or(b'0');
        hex.push(char::from(hi));
        hex.push(char::from(lo));
    }
    Ok(hex)
}
