//! Unit tests for the task list's header meters: segment colours, the
//! caption over the bar, grouped core bars, bars without figures, and the
//! Tasks/Load/Uptime line losing fields from the right.

use std::time::Duration;

use ratatui::style::Color;
use ratatui::text::Line;

use super::meters::summary;
use crate::models::{CpuCounters, MemoryBytes, MemorySample, Meters, SwapSample, SystemSample};
use crate::prompt::Privilege;
use crate::test_support::processes::{sample, system};
use crate::test_support::task_list::TaskApp;
use crate::ui::palette;
use crate::ui::Cells;

/// CPU counters in whole seconds, one per part the meter draws.
pub(super) const fn parts(user: u64, nice: u64, system: u64, idle: u64, guest: u64) -> CpuCounters {
    CpuCounters {
        user: Duration::from_secs(user),
        nice: Duration::from_secs(nice),
        system: Duration::from_secs(system),
        idle: Duration::from_secs(idle),
        virtualised: Duration::from_secs(guest),
    }
}

/// The meters of two samples of `cores` cores, the second `growth` on in
/// every counter, with `memory` and `swap` when given.
pub(super) fn meters_of(
    cores: usize,
    growth: CpuCounters,
    extra: impl Fn(&mut SystemSample),
) -> Meters {
    let zero = parts(0, 0, 0, 0, 0);
    let mut before = system(zero, &vec![zero; cores]);
    let mut after = system(growth, &vec![growth; cores]);
    extra(&mut before);
    extra(&mut after);
    let mut opened = TaskApp::open(Vec::new(), Privilege::User);
    opened.apply(sample(before, Vec::new()));
    opened.apply(sample(after, Vec::new()));
    opened.list().snapshot().unwrap().meters().clone()
}

/// Each cell of `line` with its foreground colour.
fn cells(line: &Line<'_>) -> Vec<(char, Option<Color>)> {
    line.spans
        .iter()
        .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style.fg)))
        .collect()
}

/// The colours of the `|` cells of `line`, in order.
fn bar_colours(line: &Line<'_>) -> Vec<Color> {
    cells(line)
        .into_iter()
        .filter(|(ch, _)| *ch == '|')
        .filter_map(|(_, colour)| colour)
        .collect()
}

#[test]
fn a_cpu_bar_draws_each_kind_of_time_in_its_colour() {
    // 2 user, 1 nice, 1 system, 1 guest of 10 seconds: 50 % busy.
    let meters = meters_of(1, parts(2, 1, 1, 5, 1), |_| {});
    let line = summary(&meters, Cells::new(78));
    let text = line.to_string();
    // 10.5 of 21 cells, rounded half up: 11.
    assert!(text.starts_with("CPU[|||||||||||     50.0%]"), "{text}");
    let colours = bar_colours(&line);
    let expected: Vec<Color> = [
        (4, palette::METER_USER),
        (2, palette::METER_NICE),
        (2, palette::METER_KERNEL),
        (3, palette::METER_VIRT),
    ]
    .into_iter()
    .flat_map(|(cells, colour)| std::iter::repeat_n(colour, cells))
    .collect();
    assert_eq!(colours, expected);
    let brackets: Vec<Option<Color>> = cells(&line)
        .into_iter()
        .filter(|(ch, _)| *ch == '[' || *ch == ']')
        .map(|(_, colour)| colour)
        .collect();
    assert!(brackets
        .iter()
        .all(|colour| *colour == Some(palette::FRAME)));
}

#[test]
fn the_caption_covers_the_bar_where_they_meet() {
    let meters = meters_of(1, parts(10, 0, 0, 0, 0), |_| {});
    let line = summary(&meters, Cells::new(78));
    let full = format!("CPU[{}100%]", "|".repeat(17));
    assert!(line.to_string().starts_with(&full), "{line}");
}

fn memory(total: u64, free: u64, buffers: u64, cached: u64, shared: u64) -> MemorySample {
    let kib = |kib| MemoryBytes::from_kib(kib).unwrap();
    MemorySample {
        total: kib(total),
        free: kib(free),
        available: kib(free),
        buffers: kib(buffers),
        cached: kib(cached),
        reclaimable: kib(0),
        shared: kib(shared),
    }
}

/// `colours` as runs of one colour and their lengths.
fn runs(colours: &[Color]) -> Vec<(Color, usize)> {
    let mut runs: Vec<(Color, usize)> = Vec::new();
    for &colour in colours {
        match runs.last_mut() {
            Some((last, count)) if *last == colour => *count += 1,
            _ => runs.push((colour, 1)),
        }
    }
    runs
}

#[test]
fn the_memory_and_swap_bars_draw_their_segments_in_htops_colours() {
    let meters = meters_of(1, parts(1, 0, 0, 1, 0), |system| {
        // 1000 KiB: 200 used, 100 buffers, 100 shared, 200 cache, 400 free.
        system.memory = Some(memory(1_000, 400, 100, 300, 100));
        system.swap = Some(SwapSample {
            total: MemoryBytes::from_kib(1_000).unwrap(),
            free: MemoryBytes::from_kib(500).unwrap(),
            cached: MemoryBytes::from_kib(250).unwrap(),
        });
    });
    // Three bars of 66 cells: 61 inside each.
    let line = summary(&meters, Cells::new(200));
    let text = line.to_string();
    assert!(text.contains("200 K/1.0 M] Swp["), "{text}");
    assert!(text.ends_with("250 K/1.0 M]"), "{text}");
    let colours = bar_colours(&line);
    // The CPU bar first: 50 % of 61 cells, rounded half up.
    let (cpu, rest) = colours.split_at(31);
    assert!(cpu.iter().all(|colour| *colour == palette::METER_USER));
    let (mem, swap) = rest.split_at(37);
    assert_eq!(
        runs(mem),
        [
            (palette::METER_USER, 12),
            (palette::METER_NICE, 6),
            (palette::METER_SHARED, 6),
            (palette::METER_CACHE, 13)
        ]
    );
    assert_eq!(
        runs(swap),
        [(palette::METER_KERNEL, 15), (palette::METER_CACHE, 16)]
    );
}

#[test]
fn bars_without_figures_are_empty_and_captioned_with_a_dash() {
    let meters = meters_of(1, parts(1, 0, 0, 1, 0), |_| {});
    let text = summary(&meters, Cells::new(78)).to_string();
    assert!(text.contains(" Mem[                   -]"), "{text}");
    assert!(text.ends_with(" Swp[                   -]"), "{text}");
    let mut first = TaskApp::open(Vec::new(), Privilege::User);
    let zero = parts(0, 0, 0, 0, 0);
    first.apply(sample(system(zero, &[zero]), Vec::new()));
    let pending = first.list().snapshot().unwrap().meters().clone();
    let text = summary(&pending, Cells::new(78)).to_string();
    assert!(text.starts_with("CPU[                    -]"), "{text}");
}
