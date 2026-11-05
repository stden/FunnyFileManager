//! Purpose: where blocks start. Every window of `width` lines is hashed with a
//! rolling hash, and each pair of windows with the same text becomes a block
//! when the lines before them differ: so a block is found once, at its full length.

use std::{collections::BTreeMap, time::Instant};

use super::lines::{line_before, same_line, same_lines, Line, Place, BASE};

/// Two places that start the same block, and the block's length in lines.
pub(super) struct Block {
    pub(super) first: Place,
    pub(super) second: Place,
    pub(super) lines: usize,
}

/// How far the pairing has got, and when it must stop. The clock is read once
/// per `CLOCK_EVERY` pairs: a search stays cheap, and a run of identical lines,
/// which pairs with itself at every shift, still stops close to its deadline.
struct Clock {
    deadline: Instant,
    pairs: usize,
}

const CLOCK_EVERY: usize = 256;

impl Clock {
    fn expired(&mut self) -> bool {
        self.pairs += 1;
        self.pairs.is_multiple_of(CLOCK_EVERY) && Instant::now() >= self.deadline
    }
}

/// Every block of `width` or more lines that two places start, and whether the
/// pairing reached its end before `deadline` passed.
pub(super) fn blocks(files: &[Vec<Line>], width: usize, deadline: Instant) -> (Vec<Block>, bool) {
    let mut found = Vec::new();
    let mut clock = Clock { deadline, pairs: 0 };
    for places in windows_by_hash(files, width).values() {
        if Instant::now() >= deadline || !scan_places(files, width, places, &mut clock, &mut found)
        {
            return (found, false);
        }
    }
    (found, true)
}

/// Pairs every two places of one window hash. Returns false when the clock
/// says the deadline has passed before all the pairs were looked at.
fn scan_places(
    files: &[Vec<Line>],
    width: usize,
    places: &[Place],
    clock: &mut Clock,
    found: &mut Vec<Block>,
) -> bool {
    for (position, &first) in places.iter().enumerate() {
        for &second in places.iter().skip(position + 1) {
            if clock.expired() {
                return false;
            }
            if let Some(block) = block_of_pair(files, width, first, second) {
                found.push(block);
            }
        }
    }
    true
}

/// The places of every window of `width` lines, grouped by the window's hash.
fn windows_by_hash(files: &[Vec<Line>], width: usize) -> BTreeMap<u64, Vec<Place>> {
    let mut places: BTreeMap<u64, Vec<Place>> = BTreeMap::new();
    for (source, lines) in files.iter().enumerate() {
        let hashes: Vec<u64> = lines.iter().map(|line| line.hash).collect();
        for (start, hash) in window_hashes(&hashes, width).into_iter().enumerate() {
            places.entry(hash).or_default().push((source, start));
        }
    }
    places
}

/// The hash of each run of `width` consecutive hashes, in constant time each,
/// from prefix hashes: a run's hash is its prefix less the earlier prefix scaled.
fn window_hashes(hashes: &[u64], width: usize) -> Vec<u64> {
    let prefix: Vec<u64> = std::iter::once(0_u64)
        .chain(hashes.iter().scan(0_u64, |acc, &hash| {
            *acc = acc.wrapping_mul(BASE).wrapping_add(hash);
            Some(*acc)
        }))
        .collect();
    let scale = BASE.wrapping_pow(u32::try_from(width).unwrap_or(u32::MAX));
    prefix
        .iter()
        .skip(width)
        .zip(&prefix)
        .map(|(end, start)| end.wrapping_sub(start.wrapping_mul(scale)))
        .collect()
}

/// The block two places start, unless the lines before them match too: then
/// an earlier pair starts that block, and this one would only repeat it.
fn block_of_pair(files: &[Vec<Line>], width: usize, first: Place, second: Place) -> Option<Block> {
    if !same_lines(files, first, second, width) || extends_back(files, first, second) {
        return None;
    }
    let mut lines = width;
    while same_line(files, first, second, lines) {
        lines += 1;
    }
    Some(Block {
        first,
        second,
        lines,
    })
}

/// Whether the lines before both places are the same.
fn extends_back(files: &[Vec<Line>], first: Place, second: Place) -> bool {
    match (line_before(files, first), line_before(files, second)) {
        (Some(left), Some(right)) => left.text == right.text,
        _ => false,
    }
}
