//! Cursor-navigation throughput: cost of advancing the panel cursor repeatedly.
//!
//! `run_app_with_event_reader` requires `Backend + Write` (e.g.
//! `WritableTestBackend` from the `test-helpers` feature), which bench targets
//! do not enable. These benchmarks therefore call `Panel::next` directly —
//! the inner step the event loop performs for each Down key — approximating
//! event throughput without the full terminal-draw round-trip.
use std::{hint::black_box, path::PathBuf, time::Duration};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use funny_file_manager::models::{App, DirEntry, EntryKind, FileSize, Modified, Panel};

fn entry(index: usize) -> DirEntry {
    DirEntry {
        name: format!("file-{index:05}.txt"),
        kind: if index.is_multiple_of(10) {
            EntryKind::Directory
        } else {
            EntryKind::File
        },
        size: FileSize::from_bytes(u64::try_from(index).unwrap_or(0) * 512),
        modified: Modified::UNKNOWN,
        ..DirEntry::default()
    }
}

fn app_with_entries(count: usize) -> App {
    let entries: Vec<DirEntry> = (0..count).map(entry).collect();
    let path = PathBuf::from("/tmp/bench");
    App::with_panels(
        Panel::from_entries(path.clone(), entries.clone()),
        Panel::from_entries(path, entries),
    )
}

/// Dispatch 128 Down steps on the panel model in a single iteration.
///
/// Approximates `bench_key_burst` with 128 Down key events routed through the
/// full event loop; the actual per-key cost here is just `Panel::next`, so
/// the wall-clock difference between this and a full-dispatch version names
/// the overhead of the dispatch and draw layers.
fn bench_key_burst(c: &mut Criterion) {
    let mut app = app_with_entries(256);
    c.bench_function("event/key_burst_128", |b| {
        b.iter(|| {
            for _ in 0..128 {
                app.left_panel.next();
            }
            black_box(app.left_panel.get_selected_entry());
        });
    });
}

/// Cost of a single cursor-down step.
fn bench_single_key(c: &mut Criterion) {
    let sizes = [100_usize, 1_000, 10_000];
    let mut group = c.benchmark_group("event/single_key_down");
    for size in sizes {
        let mut app = app_with_entries(size);
        group.bench_function(BenchmarkId::from_parameter(size), |b| {
            b.iter(|| {
                app.left_panel.next();
                black_box(app.left_panel.get_selected_entry());
            });
        });
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets = bench_key_burst, bench_single_key
}
criterion_main!(benches);
