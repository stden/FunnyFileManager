//! Purpose: State for Commands > Space analysis: the active folder's entries as
//! verdicts, largest first, and the cursor on them. Nothing here touches the disk.

use std::path::PathBuf;

use super::FileSize;
use crate::space_verdict::Verdict;

/// Entries the dialog lists, largest first.
pub const SHOWN: usize = 25;

/// Rows the cursor moves by on `PgUp` and `PgDn`.
pub const PAGE: usize = 10;

/// One entry of the folder, judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceEntry {
    pub name: String,
    pub bytes: u64,
    pub verdict: Verdict,
}

/// One listed row: an entry, with its share of the folder total.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceRow {
    pub name: String,
    pub bytes: u64,
    /// Whole percent of the folder total.
    pub share: u64,
    pub verdict: Verdict,
}

/// The Space analysis dialog.
#[derive(Debug, Clone)]
pub struct SpaceAnalysisDialog {
    pub folder: PathBuf,
    /// Every entry of the folder, not only the listed ones.
    pub total: FileSize,
    /// The sum of every SAFE entry.
    pub reclaimable: FileSize,
    /// What limited the measurement, and what git could not say.
    pub note: String,
    pub rows: Vec<SpaceRow>,
    pub cursor: usize,
}

impl SpaceAnalysisDialog {
    /// Totals over every entry; the rows are the largest [`SHOWN`], by name on ties.
    #[must_use]
    pub fn new(folder: PathBuf, mut entries: Vec<SpaceEntry>, note: String) -> Self {
        let total = entries
            .iter()
            .fold(0_u64, |sum, entry| sum.saturating_add(entry.bytes));
        let reclaimable = entries
            .iter()
            .filter(|entry| matches!(entry.verdict, Verdict::Safe(_)))
            .fold(0_u64, |sum, entry| sum.saturating_add(entry.bytes));
        entries.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
        entries.truncate(SHOWN);
        let rows = entries
            .into_iter()
            .map(|entry| SpaceRow {
                share: share_percent(entry.bytes, total),
                name: entry.name,
                bytes: entry.bytes,
                verdict: entry.verdict,
            })
            .collect();
        Self {
            folder,
            total: FileSize::from_bytes(total),
            reclaimable: FileSize::from_bytes(reclaimable),
            note,
            rows,
            cursor: 0,
        }
    }

    /// The name under the cursor, when there is a row.
    #[must_use]
    pub fn selected_name(&self) -> Option<&str> {
        self.rows.get(self.cursor).map(|row| row.name.as_str())
    }

    /// Put the cursor on row `index`, or on the last row when it is past them.
    pub fn move_to(&mut self, index: usize) {
        self.cursor = index.min(self.rows.len().saturating_sub(1));
    }
}

/// `bytes` as a whole percentage of `total`; zero for an empty folder.
#[must_use]
pub fn share_percent(bytes: u64, total: u64) -> u64 {
    if total == 0 {
        return 0;
    }
    u64::try_from(u128::from(bytes) * 100 / u128::from(total)).unwrap_or(100)
}

#[cfg(test)]
mod tests;
