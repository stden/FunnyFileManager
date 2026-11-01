//! Purpose: the cheap per-frame helpers — layout geometry and the clock.

use criterion::Criterion;
use funny_file_manager::ui::{panel_page_size, FKEY_COUNT};
use ratatui::layout::Rect;
use std::hint::black_box;

/// The pure geometry, called several times a frame.
pub fn bench_layout(c: &mut Criterion) {
    let mut group = c.benchmark_group("layout");
    let screen = Rect::new(0, 0, 120, 40);

    group.bench_function("fkey_widths", |b| {
        b.iter(|| funny_file_manager::mouse::fkey_widths(black_box(120), black_box(FKEY_COUNT)));
    });

    group.bench_function("mouse_hit", |b| {
        b.iter(|| {
            funny_file_manager::mouse::hit(
                black_box(screen),
                black_box(42),
                black_box(12),
                black_box(FKEY_COUNT),
                black_box(funny_file_manager::models::PanelsShown::Both),
            )
        });
    });

    group.bench_function("panel_page_size", |b| {
        b.iter(|| panel_page_size(black_box(40)));
    });

    group.bench_function("menu_heading_at", |b| {
        b.iter(|| funny_file_manager::ui::heading_at(black_box(screen), black_box(12)));
    });

    group.finish();
}

/// The clock, which the bar reads once per frame.
///
/// Its own benchmark because it is the one part of a frame that has ever
/// reached outside the process, and a frame is drawn on every keypress.
pub fn bench_clock(c: &mut Criterion) {
    c.bench_function("clock/now", |b| b.iter(funny_file_manager::clock::now));
}
