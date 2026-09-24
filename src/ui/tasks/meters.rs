//! Purpose: The task list's header — the CPU, Mem and Swp bars, the per-core
//! bars and the Tasks/Load/Uptime line — as spans, htop's meters drawn in
//! Far's colours.
//!
//! A bar is its label, `[`, the coloured segments and the caption flush
//! right, then `]`. Each segment ends at its running total's share of the
//! bar, rounded once, so the segments never add up to more than the bar.

use std::num::{NonZeroU128, NonZeroUsize};

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::models::{
    CoreGroup, CpuMeter, MemoryBytes, MemoryMeter, Meters, Percent, SwapMeter, BARS_PER_ROW,
};

use super::super::panel::right_cells;
use super::super::{head_cells, palette, Cells};
use super::format;

/// Blank cells between two bars on one row.
const GAP: &str = " ";
/// The narrowest label a core bar gets, so single digits line up under
/// double ones.
const MIN_CORE_LABEL: Cells = Cells::new(2);
/// Blank cells between two fields of the Tasks/Load/Uptime line.
const FIELD_GAP: &str = "  ";

/// One share of a bar: how much of the whole, and its colour.
type Segment = (u128, Color);

/// A bar's contents before it is laid out.
struct Bar {
    label: String,
    segments: Vec<Segment>,
    whole: u128,
    caption: String,
}

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

fn cpu_bar(label: &str, meter: CpuMeter) -> Bar {
    let segments = match meter {
        CpuMeter::Pending => Vec::new(),
        CpuMeter::Measured(bar) => vec![
            share(bar.user(), palette::METER_USER),
            share(bar.nice(), palette::METER_NICE),
            share(bar.kernel(), palette::METER_KERNEL),
            share(bar.virtualised(), palette::METER_VIRT),
        ],
    };
    Bar {
        label: label.to_owned(),
        segments,
        whole: u128::from(Percent::FULL.hundredths()),
        caption: format::cpu_meter(meter),
    }
}

fn share(part: Percent, colour: Color) -> Segment {
    (u128::from(part.hundredths()), colour)
}

fn core_bar(group: CoreGroup) -> Bar {
    let label = if group.first_core() == group.last_core() {
        group.first_core().to_string()
    } else {
        format!("{}-{}", group.first_core(), group.last_core())
    };
    cpu_bar(&label, group.meter())
}

fn memory_bar(memory: MemoryMeter) -> Bar {
    Bar {
        label: "Mem".to_owned(),
        segments: vec![
            (bytes(memory.used()), palette::METER_USER),
            (bytes(memory.buffers()), palette::METER_NICE),
            (bytes(memory.shared()), palette::METER_SHARED),
            (bytes(memory.cache()), palette::METER_CACHE),
        ],
        whole: bytes(memory.total()),
        caption: format::used_of(memory.used(), memory.total()),
    }
}

fn swap_bar(swap: SwapMeter) -> Bar {
    Bar {
        label: "Swp".to_owned(),
        segments: vec![
            (bytes(swap.used()), palette::METER_KERNEL),
            (bytes(swap.cache()), palette::METER_CACHE),
        ],
        whole: bytes(swap.total()),
        caption: format::used_of(swap.used(), swap.total()),
    }
}

fn bytes(size: MemoryBytes) -> u128 {
    u128::from(size.bytes())
}

/// A meter the platform gave no figures for: an empty bar captioned `-`.
fn unknown_bar(label: &str) -> Bar {
    Bar {
        label: label.to_owned(),
        segments: Vec::new(),
        whole: 0,
        caption: format::UNKNOWN.to_owned(),
    }
}

impl Bar {
    /// The bar in exactly `width` cells, its label right-aligned in `label`.
    fn spans(&self, label: Cells, width: Cells) -> Vec<Span<'static>> {
        let frame = Style::default().fg(palette::FRAME);
        let inner = width.saturating_sub(label + Cells::new(2));
        let mut spans = vec![
            Span::styled(right_cells(&self.label, label), frame),
            Span::styled("[", frame),
        ];
        spans.extend(self.inner(inner));
        spans.push(Span::styled("]", frame));
        spans
    }

    /// The inside of the bar: each segment's `|` in its colour, the caption
    /// flush right over whatever it covers.
    fn inner(&self, width: Cells) -> Vec<Span<'static>> {
        let caption = head_cells(&self.caption, width);
        let caption_at = width.saturating_sub(Cells::of(&caption)).get();
        let mut spans = Vec::new();
        let mut drawn = 0_usize;
        for (end, colour) in self.ends(width.get()) {
            let end = end.min(caption_at);
            let cells = end.saturating_sub(drawn);
            if cells > 0 {
                spans.push(Span::styled("|".repeat(cells), Style::default().fg(colour)));
            }
            drawn = drawn.max(end);
        }
        spans.push(Span::raw(" ".repeat(caption_at.saturating_sub(drawn))));
        spans.push(Span::styled(caption, Style::default().fg(palette::DATA)));
        spans
    }

    /// Where each segment ends, in cells of a bar `cells` wide: its running
    /// total's share, rounded half up and never past the bar.
    fn ends(&self, cells: usize) -> Vec<(usize, Color)> {
        let Some(whole) = NonZeroU128::new(self.whole) else {
            return Vec::new();
        };
        let wide = u128::try_from(cells).unwrap_or(u128::MAX);
        let mut reached = 0_u128;
        self.segments
            .iter()
            .map(|&(part, colour)| {
                reached = reached.saturating_add(part).min(whole.get());
                let scaled = reached.saturating_mul(wide).saturating_add(whole.get() / 2);
                let end = usize::try_from(scaled / whole).unwrap_or(cells).min(cells);
                (end, colour)
            })
            .collect()
    }
}
