//! Purpose: the groups the blocks form. Blocks whose text is the same are one
//! group; a place that overlaps an earlier one in its text is dropped; and a
//! group needs two places left.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use super::lines::{block_hash, kept_lines, line_at, same_lines, Line, Place};
use super::windows::{self, Block};
use super::{Group, Occurrence};

/// The places one block text occurs at, and the place its first copy starts at.
struct Candidate {
    start: Place,
    lines: usize,
    places: BTreeSet<Place>,
}

/// The groups of the texts, and whether the pairing finished before `deadline`.
pub(super) fn repeated_blocks(
    texts: &[&str],
    width: usize,
    deadline: Instant,
) -> (Vec<Group>, bool) {
    let files: Vec<Vec<Line>> = texts.iter().map(|text| kept_lines(text)).collect();
    let (blocks, complete) = windows::blocks(&files, width, deadline);
    let mut candidates: BTreeMap<(u64, usize), Vec<Candidate>> = BTreeMap::new();
    for block in blocks {
        record(&mut candidates, &files, &block);
    }
    let groups = candidates
        .into_values()
        .flatten()
        .filter_map(|candidate| group(&files, &candidate))
        .collect();
    (groups, complete)
}

/// Files a block under its text: the same text, found again, adds its places.
fn record(
    candidates: &mut BTreeMap<(u64, usize), Vec<Candidate>>,
    files: &[Vec<Line>],
    block: &Block,
) {
    let found = candidates
        .entry((block_hash(files, block.first, block.lines), block.lines))
        .or_default();
    let existing = found
        .iter()
        .position(|candidate| same_lines(files, candidate.start, block.first, block.lines));
    match existing.and_then(|position| found.get_mut(position)) {
        Some(candidate) => candidate.places.extend([block.first, block.second]),
        None => found.push(Candidate {
            start: block.first,
            lines: block.lines,
            places: BTreeSet::from([block.first, block.second]),
        }),
    }
}

/// The group a candidate makes, or nothing when fewer than two of its places
/// survive: a place that overlaps an earlier one in the same text is the same
/// block shifted by a line or more, and is not a second copy.
fn group(files: &[Vec<Line>], candidate: &Candidate) -> Option<Group> {
    let kept = non_overlapping(&candidate.places, candidate.lines);
    if kept.len() < 2 {
        return None;
    }
    let occurrences = kept
        .into_iter()
        .filter_map(|(source, start)| occurrence(files, source, start, candidate.lines))
        .collect();
    Some(Group {
        block_lines: candidate.lines,
        occurrences,
    })
}

fn non_overlapping(places: &BTreeSet<Place>, lines: usize) -> Vec<Place> {
    let mut kept: Vec<Place> = Vec::new();
    for &(source, start) in places {
        let overlaps = kept.last().is_some_and(|&(last_source, last_start)| {
            last_source == source && start < last_start + lines
        });
        if !overlaps {
            kept.push((source, start));
        }
    }
    kept
}

fn occurrence(
    files: &[Vec<Line>],
    source: usize,
    start: usize,
    lines: usize,
) -> Option<Occurrence> {
    let first = line_at(files, (source, start), 0)?;
    let last = line_at(files, (source, start), lines - 1)?;
    Some(Occurrence {
        source,
        first_line: first.number,
        last_line: last.number,
    })
}
