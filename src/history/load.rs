//! Purpose: Reading the durable lines back from the history file — the size guard and the re-check.

use std::collections::VecDeque;
use std::fs;

use super::DurableLine;

/// Defensive upper bound on the history file we will read into memory. We only
/// ever write `HISTORY_CAP` (500) lines of paths — far under a mebibyte — so a
/// file larger than this is hand-made or corrupt; rather than slurp it whole
/// with `read_to_string`, we treat it as empty (completion must never wedge the
/// file manager over a bad history file).
const MAX_HISTORY_FILE_BYTES: u64 = 1 << 20;

/// The durable lines of the history file at `path`. Missing, unreadable,
/// oversize or corrupt files are an empty history — completion must never
/// break the file manager.
pub(super) fn read_durable(path: &std::path::Path) -> VecDeque<DurableLine> {
    // Skip the read entirely for an oversize (hand-made / corrupt) file.
    if !path
        .metadata()
        .is_ok_and(|meta| meta.len() <= MAX_HISTORY_FILE_BYTES)
    {
        return VecDeque::new();
    }
    fs::read_to_string(path)
        .map(|text| {
            text.lines()
                // classify() re-checks on load: a hand-edited file cannot
                // smuggle a shell line back into the durable set.
                .filter_map(DurableLine::classify)
                .collect()
        })
        .unwrap_or_default()
}
