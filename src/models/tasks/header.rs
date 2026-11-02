//! Purpose: How many rows the task list's header takes on a screen of a given
//! height — the summary rows, the per-core bars and how many cores each bar
//! averages — and so how many table rows are left: one answer for the
//! renderer, for paging and for the mouse.
//!
//! In the models rather than `ui` because paging needs it and the models may
//! not depend on the presentation layer. It knows only the terminal's height
//! and the core count, never the width.

use std::num::NonZeroUsize;

use crate::models::Viewport;

use super::{Snapshot, TaskList};

/// Rows every layout draws: the top border, the column titles, the closing
/// rule, the info row, the bottom border and the key bar.
const FRAME_CHROME: u16 = 6;
/// The rule between the meters and the column titles, drawn only under
/// meters.
const METER_RULE: u16 = 1;
/// The CPU/Mem/Swp row and the Tasks/Load/Uptime row.
const SUMMARY_ROWS: u16 = 2;
/// Table rows the meters never take: below this many the meters go.
const MIN_TABLE_ROWS: u16 = 5;
/// Core bars side by side on one header row.
pub const BARS_PER_ROW: usize = 4;

/// The screen height at which the summary rows first fit beside the table.
const METERS_FROM: u16 = FRAME_CHROME + METER_RULE + SUMMARY_ROWS + MIN_TABLE_ROWS;

/// The header of a task list drawn on one screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeaderShape {
    summary_rows: u16,
    cpu_rows: u16,
    /// `None` when no core bars are drawn.
    cores_per_bar: Option<NonZeroUsize>,
    chrome: u16,
    page: usize,
}

impl HeaderShape {
    /// The header for `cores` per-core counters on a terminal `viewport`
    /// tall. `cores` is 0 until the first sample arrives.
    #[must_use]
    pub fn for_screen(cores: usize, viewport: Viewport) -> Self {
        let rows = viewport.rows();
        let summary_rows = if rows >= METERS_FROM { SUMMARY_ROWS } else { 0 };
        let budget = if summary_rows == 0 || cores <= 1 {
            0
        } else {
            (rows / 3)
                .saturating_sub(SUMMARY_ROWS)
                .min(rows.saturating_sub(METERS_FROM))
        };
        let (cpu_rows, cores_per_bar) = core_bars(cores, budget);
        let meters = summary_rows.saturating_add(cpu_rows);
        let rule = if meters > 0 { METER_RULE } else { 0 };
        let chrome = FRAME_CHROME.saturating_add(meters).saturating_add(rule);
        Self {
            summary_rows,
            cpu_rows,
            cores_per_bar,
            chrome,
            page: usize::from(rows.saturating_sub(chrome)).max(1),
        }
    }

    /// The CPU/Mem/Swp and Tasks/Load/Uptime rows: 2, or 0 on a short
    /// screen.
    #[must_use]
    pub const fn summary_rows(self) -> u16 {
        self.summary_rows
    }

    /// Rows of core bars, [`BARS_PER_ROW`] to a row.
    #[must_use]
    pub const fn cpu_rows(self) -> u16 {
        self.cpu_rows
    }

    /// How many neighbouring cores each bar averages, for
    /// `CpuMeters::grouped`; `None` when no core bars are drawn.
    #[must_use]
    pub const fn cores_per_bar(self) -> Option<NonZeroUsize> {
        self.cores_per_bar
    }

    /// Every row that is not a table row.
    #[must_use]
    pub const fn chrome(self) -> u16 {
        self.chrome
    }

    /// Table rows: what `PgUp` and `PgDn` move by. Never zero; on a screen too
    /// small for even that the renderer clips.
    #[must_use]
    pub const fn page(self) -> usize {
        self.page
    }
}

/// Rows of core bars for `cores` in at most `budget` rows, and how many
/// cores share a bar so they fit: none without a budget.
fn core_bars(cores: usize, budget: u16) -> (u16, Option<NonZeroUsize>) {
    let Some(slots) = NonZeroUsize::new(BARS_PER_ROW.saturating_mul(usize::from(budget))) else {
        return (0, None);
    };
    let Some(per_bar) = NonZeroUsize::new(cores.div_ceil(slots.get())) else {
        return (0, None);
    };
    let bars = cores.div_ceil(per_bar.get());
    // At most `slots` bars, so at most `budget` rows.
    let rows = u16::try_from(bars.div_ceil(BARS_PER_ROW)).map_or(budget, |rows| rows.min(budget));
    (rows, Some(per_bar))
}

impl TaskList {
    /// Per-core counters in the snapshot: 0 until the first sample.
    #[must_use]
    pub fn cores(&self) -> usize {
        self.snapshot().map_or(0, Snapshot::cores)
    }

    /// The header on a terminal `viewport` tall.
    #[must_use]
    pub fn header_shape(&self, viewport: Viewport) -> HeaderShape {
        HeaderShape::for_screen(self.cores(), viewport)
    }

    /// Table rows on a terminal `viewport` tall: what `PgUp` and `PgDn` move by.
    #[must_use]
    pub fn page_for(&self, viewport: Viewport) -> usize {
        self.header_shape(viewport).page()
    }

    /// Keeps the scroll offset showing the cursor after a resize.
    pub fn clamp_to(&mut self, viewport: Viewport) {
        let page = self.page_for(viewport);
        self.selection.clamp(self.view.len(), page);
    }
}

#[cfg(test)]
mod tests_cores;
