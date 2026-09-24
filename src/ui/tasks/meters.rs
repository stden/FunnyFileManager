//! Purpose: The task list's header — the CPU, Mem and Swp bars, the per-core
//! bars and the Tasks/Load/Uptime line — as spans, htop's meters drawn in
//! Far's colours.
//!
//! A bar is its label, `[`, the coloured segments and the caption flush
//! right, then `]`. Each segment ends at its running total's share of the
//! bar, rounded once, so the segments never add up to more than the bar.

mod bar;

use std::num::NonZeroUsize;

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::models::{Meters, BARS_PER_ROW};

use super::super::{palette, Cells};
use super::format;
use bar::{core_bar, cpu_bar, memory_bar, swap_bar, unknown_bar, Bar};

/// Blank cells between two bars on one row.
const GAP: &str = " ";
/// The narrowest label a core bar gets, so single digits line up under
/// double ones.
const MIN_CORE_LABEL: Cells = Cells::new(2);
/// Blank cells between two fields of the Tasks/Load/Uptime line.
const FIELD_GAP: &str = "  ";

/// The first summary row: CPU, Mem and Swp in thirds.
pub(super) fn summary(meters: &Meters, width: Cells) -> Line<'static> {
    let bars = [
        cpu_bar("CPU", meters.cpu()),
        meters
            .memory()
            .map_or_else(|| unknown_bar("Mem"), memory_bar),
        meters.swap().map_or_else(|| unknown_bar("Swp"), swap_bar),
    ];
    row(&bars, bars.len(), width, None)
}

/// The per-core rows: `per_bar` neighbouring cores to a bar, four bars to a
/// row, `rows` rows.
pub(super) fn cores(
    meters: &Meters,
    per_bar: NonZeroUsize,
    rows: u16,
    width: Cells,
) -> Vec<Line<'static>> {
    let bars: Vec<Bar> = meters
        .cores()
        .grouped(per_bar)
        .into_iter()
        .map(core_bar)
        .collect();
    let label = bars
        .iter()
        .map(|bar| Cells::of(&bar.label))
        .fold(MIN_CORE_LABEL, Cells::max);
    bars.chunks(BARS_PER_ROW)
        .take(usize::from(rows))
        .map(|chunk| row(chunk, BARS_PER_ROW, width, Some(label)))
        .collect()
}

/// The second summary row: `Tasks`, `Load average` and `Uptime`, two blanks
/// apart; fields that do not fit are dropped from the right.
pub(super) fn tasks(meters: &Meters, width: Cells) -> Line<'static> {
    let fields = [
        Some(format::tasks(meters.tasks())),
        meters.load().map(format::load),
        meters.uptime().map(format::uptime),
    ];
    let mut text = String::new();
    for field in fields.into_iter().flatten() {
        let joined = if text.is_empty() {
            field
        } else {
            format!("{text}{FIELD_GAP}{field}")
        };
        if Cells::of(&joined) > width {
            break;
        }
        text = joined;
    }
    Line::from(Span::styled(text, Style::default().fg(palette::DATA)))
}

/// `bars` in the first of `slots` equal slots across `width` — a row of core
/// bars keeps four slots even when it holds fewer bars, so the bars line up
/// down the rows. Each label is right-aligned in `label` cells, or in its
/// own width.
fn row(bars: &[Bar], slots: usize, width: Cells, label: Option<Cells>) -> Line<'static> {
    let widths = slot_widths(width, slots);
    let spans = bars.iter().zip(widths).map(|(bar, slot)| {
        let label = label.unwrap_or_else(|| Cells::of(&bar.label));
        bar.spans(label, slot)
    });
    Line::from(joined(spans.collect()))
}

/// `count` slot widths sharing `width` less the gaps, the remainder going
/// to the left, as the key bar shares its row.
fn slot_widths(width: Cells, count: usize) -> Vec<Cells> {
    let gaps = count.saturating_sub(1).saturating_mul(Cells::of(GAP).get());
    let room = u16::try_from(width.get().saturating_sub(gaps)).unwrap_or(u16::MAX);
    crate::mouse::fkey_widths(room, count)
        .into_iter()
        .map(Cells::columns)
        .collect()
}

/// The spans of each bar, a gap between two.
fn joined(bars: Vec<Vec<Span<'static>>>) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (index, bar) in bars.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(GAP));
        }
        spans.extend(bar);
    }
    spans
}
