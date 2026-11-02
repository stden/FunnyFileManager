//! Purpose: Reading the durable lines back from the history file — the size guard and the re-check.

use std::collections::VecDeque;

use super::DurableLine;

/// The durable lines of the history file at `path`. Missing, unreadable,
/// oversize or corrupt files are an empty history — completion must never
/// break the file manager. We only ever write `HISTORY_CAP` (500) lines of
/// paths, far under the mebibyte a data file may have; and a FIFO in the
/// file's place is refused rather than waited on.
pub(super) fn read_durable(path: &std::path::Path) -> VecDeque<DurableLine> {
    crate::fileload::read_small_file(path, "history")
        .map(|text| {
            text.lines()
                // classify() re-checks on load: a hand-edited file cannot
                // smuggle a shell line back into the durable set.
                .filter_map(DurableLine::classify)
                .collect()
        })
        .unwrap_or_default()
}
