//! Purpose: One meter bar of the task list's header — its label, coloured
//! segments and caption — built from a CPU, memory or swap reading and laid
//! out in cells, each segment ending at its running total's share.

use std::num::NonZeroU128;

use ratatui::style::{Color, Style};
use ratatui::text::Span;

use crate::models::{CoreGroup, CpuMeter, MemoryBytes, MemoryMeter, Percent, SwapMeter};

use super::super::super::panel::right_cells;
use super::super::super::{head_cells, palette, Cells};
use super::super::format;

/// One share of a bar: how much of the whole, and its colour.
type Segment = (u128, Color);

/// A bar's contents before it is laid out.
pub(super) struct Bar {
    pub(super) label: String,
    segments: Vec<Segment>,
    whole: u128,
    caption: String,
}

pub(super) fn cpu_bar(label: &str, meter: CpuMeter) -> Bar {
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

pub(super) fn core_bar(group: CoreGroup) -> Bar {
    let label = if group.first_core() == group.last_core() {
        group.first_core().to_string()
    } else {
        format!("{}-{}", group.first_core(), group.last_core())
    };
    cpu_bar(&label, group.meter())
}

pub(super) fn memory_bar(memory: MemoryMeter) -> Bar {
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

pub(super) fn swap_bar(swap: SwapMeter) -> Bar {
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
pub(super) fn unknown_bar(label: &str) -> Bar {
    Bar {
        label: label.to_owned(),
        segments: Vec::new(),
        whole: 0,
        caption: format::UNKNOWN.to_owned(),
    }
}

impl Bar {
    /// The bar in exactly `width` cells, its label right-aligned in `label`.
    pub(super) fn spans(&self, label: Cells, width: Cells) -> Vec<Span<'static>> {
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
