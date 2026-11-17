//! Purpose: Single-pass streaming byte comparison with integrated SHA-256 hashing.

use crate::checksum::{finish_reader_hash, hex_digest};
use crate::conversions::saturating_u64;
use sha2::{Digest, Sha256};
use std::io::{self, BufRead};

pub(super) struct PairDigests {
    pub(super) left_sha256: String,
    pub(super) right_sha256: String,
}

pub(super) enum ComparisonOutcome {
    Identical(String),
    Different {
        offset: u64,
        digests: Option<PairDigests>,
    },
}

pub(super) fn compare_streams<L: io::Read, R: io::Read>(
    left: L,
    right: R,
    left_size: u64,
    right_size: u64,
) -> io::Result<ComparisonOutcome> {
    let equal_size = left_size == right_size;
    let cap = stream_buffer_cap(left_size.max(right_size));
    let mut left = io::BufReader::with_capacity(cap, left);
    let mut right = io::BufReader::with_capacity(cap, right);
    let mut hasher = Sha256::new();
    let mut offset = 0u64;

    loop {
        let (left_block, right_block) = (left.fill_buf()?, right.fill_buf()?);
        if left_block.is_empty() && right_block.is_empty() {
            return Ok(ComparisonOutcome::Identical(hex_digest(hasher)));
        }
        if left_block.is_empty() || right_block.is_empty() {
            return diverged_outcome(offset, equal_size, left, right, hasher);
        }
        if let Some(diff) = first_mismatch(left_block, right_block) {
            let at = offset + saturating_u64(diff);
            return diverged_outcome(at, equal_size, left, right, hasher);
        }
        let step = left_block.len().min(right_block.len());
        update_shared_prefix(&mut hasher, equal_size, left_block, step);
        left.consume(step);
        right.consume(step);
        offset += saturating_u64(step);
    }
}

fn update_shared_prefix(hasher: &mut Sha256, equal_size: bool, block: &[u8], step: usize) {
    if equal_size {
        if let Some(prefix) = block.get(..step) {
            hasher.update(prefix);
        }
    }
}

#[inline]
fn first_mismatch(left: &[u8], right: &[u8]) -> Option<usize> {
    let common = left.len().min(right.len());
    if left.get(..common) == right.get(..common) {
        return None;
    }
    left.iter().zip(right).position(|(a, b)| a != b)
}

fn stream_buffer_cap(max_size: u64) -> usize {
    usize::try_from(max_size)
        .unwrap_or(256 * 1024)
        .clamp(8 * 1024, 256 * 1024)
}

fn diverged_outcome<L: io::Read, R: io::Read>(
    offset: u64,
    equal_size: bool,
    left: io::BufReader<L>,
    right: io::BufReader<R>,
    hasher: Sha256,
) -> io::Result<ComparisonOutcome> {
    let digests = if equal_size {
        let right_hasher = hasher.clone();
        Some(PairDigests {
            left_sha256: finish_reader_hash(left, hasher)?,
            right_sha256: finish_reader_hash(right, right_hasher)?,
        })
    } else {
        None
    };
    Ok(ComparisonOutcome::Different { offset, digests })
}
