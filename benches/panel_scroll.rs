//! Purpose: The cost of stepping the panel cursor and redrawing one frame.
//!
//! Holding Down or `PgDn` pays this on every step. `ui_bench` draws a still
//! listing; this one moves the cursor first, which is what the frame budget
//! batches.

use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use funny_file_manager::models::{App, DirEntry, EntryKind, FileSize, Modified, Panel};
use funny_file_manager::ui::ui;
use ratatui::{backend::TestBackend, Terminal};
use tempfile::TempDir;

const SIZES: [usize; 3] = [1_000, 10_000, 100_000];

fn entry(index: usize) -> DirEntry {
    DirEntry {
        name: format!("document-{index:05}.txt"),
        kind: if index.is_multiple_of(7) {
            EntryKind::Directory
        } else {
            EntryKind::File
        },
        size: FileSize::from_bytes(u64::try_from(index).unwrap_or(0) * 1_337),
        modified: Modified::UNKNOWN,
        executable: index.is_multiple_of(11),
        ..DirEntry::default()
    }
}

fn panel(count: usize) -> Panel {
    let entries = (0..count).map(entry).collect();
    Panel::from_entries(PathBuf::from("/srv/x"), entries)
}

fn rewind(listing: &mut Panel) {
    let last = listing.entries.len().saturating_sub(1);
    if listing.selected() == Some(last) {
        listing.select(Some(0));
    }
}

fn bench_steps(c: &mut Criterion) {
    let mut group = c.benchmark_group("panel/cursor_step");
    for size in SIZES {
        let mut listing = panel(size);
        group.bench_function(BenchmarkId::new("next", size), |b| {
            b.iter(|| {
                rewind(&mut listing);
                listing.next();
                black_box(listing.selected());
            });
        });
        group.bench_function(BenchmarkId::new("page_down", size), |b| {
            b.iter(|| {
                rewind(&mut listing);
                listing.page_down(40);
                black_box(listing.selected());
            });
        });
    }
    group.finish();
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
fn app_of(count: usize) -> (App, TempDir) {
    let root = TempDir::new().expect("benchmark temp dir");
    let left = root.path().join("left");
    fs::create_dir_all(&left).expect("left panel directory");
    let entries = (0..count).map(entry).collect();
    let app = App::with_panels(
        Panel::from_entries(left, entries),
        Panel::from_entries(root.path().to_path_buf(), Vec::new()),
    );
    (app, root)
}

fn bench_page_draw(c: &mut Criterion) {
    let mut group = c.benchmark_group("panel/page_and_draw");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(200));
    group.measurement_time(Duration::from_secs(1));
    for size in SIZES {
        draw_one(&mut group, size);
    }
    group.finish();
}

fn draw_one(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    size: usize,
) {
    let (mut app, _root) = app_of(size);
    let mut term = Terminal::new(TestBackend::new(120, 40)).expect("benchmark terminal");
    group.bench_function(BenchmarkId::from_parameter(size), |b| {
        b.iter(|| {
            rewind(&mut app.left_panel);
            app.left_panel.page_down(40);
            term.draw(|frame| ui(frame, black_box(&mut app)))
                .expect("draw");
            black_box(app.left_panel.selected());
        });
    });
}

criterion_group!(benches, bench_steps, bench_page_draw);
criterion_main!(benches);
