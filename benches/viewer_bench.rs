//! Full viewer redraw at three synthetic file sizes and a mid-file scroll benchmark.
//!
//! Mirrors `editor_cursor_bench.rs`: warm the highlight cache before measuring,
//! then call `ui` in a tight loop so only the render cost is sampled.
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use funny_file_manager::{
    models::{App, Overlay, Panel, Viewer},
    scroll::ScrollTop,
    ui::ui,
};
use ratatui::{backend::TestBackend, Terminal};
use std::{hint::black_box, time::Duration};
use tempfile::TempDir;

const BENCH_LINE: &str = "let answer = 42; // bench line\n";

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
fn viewer_fixture(root: &TempDir, size_bytes: usize) -> App {
    let count = (size_bytes / BENCH_LINE.len()).max(1);
    let content = BENCH_LINE.repeat(count);
    let left_dir = root.path().join("left");
    let path = left_dir.join("viewer_source.rs");
    std::fs::write(&path, content.as_bytes()).expect("write source file");
    let viewer = Viewer::file(path, content.as_bytes());
    let mut left = Panel::new(left_dir).expect("left panel");
    left.load_entries().expect("load left panel");
    left.select_named("viewer_source.rs");
    let right = Panel::new(root.path().join("right")).expect("right panel");
    let mut app = App::with_panels(left, right);
    app.set_overlay(Overlay::Viewer(viewer));
    app
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
fn bench_viewer_render(c: &mut Criterion) {
    let root = TempDir::new().expect("temp dir");
    std::fs::create_dir(root.path().join("left")).expect("left dir");
    std::fs::create_dir(root.path().join("right")).expect("right dir");

    let mut group = c.benchmark_group("viewer/render");
    for size_bytes in [1_024_usize, 65_536, 524_288] {
        let mut app = viewer_fixture(&root, size_bytes);
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("terminal");
        // Warm the highlight cache before measuring.
        terminal
            .draw(|frame| ui(frame, &mut app))
            .expect("warm highlight cache");
        group.bench_with_input(
            BenchmarkId::from_parameter(size_bytes),
            &size_bytes,
            |b, _| {
                b.iter(|| {
                    terminal
                        .draw(|frame| ui(frame, black_box(&mut app)))
                        .expect("draw");
                    black_box(terminal.backend().buffer());
                });
            },
        );
    }
    group.finish();
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
fn bench_viewer_scroll(c: &mut Criterion) {
    let root = TempDir::new().expect("temp dir");
    std::fs::create_dir(root.path().join("left")).expect("left dir");
    std::fs::create_dir(root.path().join("right")).expect("right dir");

    let content = BENCH_LINE.repeat(10_000);
    let path = root.path().join("left").join("viewer_scroll.rs");
    std::fs::write(&path, content.as_bytes()).expect("write scroll file");
    let viewer = Viewer::file(path, content.as_bytes());
    let total_lines = viewer.lines.len();
    let left = Panel::new(root.path().join("left")).expect("left panel");
    let right = Panel::new(root.path().join("right")).expect("right panel");
    let mut app = App::with_panels(left, right);
    app.set_overlay(Overlay::Viewer(viewer));

    let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("terminal");
    terminal
        .draw(|frame| ui(frame, &mut app))
        .expect("warm highlight cache");

    // Advance the viewport to 50 % of the total line count.
    if let Overlay::Viewer(ref mut v) = app.overlay {
        let mid = total_lines / 2;
        let last = total_lines.saturating_sub(1);
        v.scroll = ScrollTop::TOP.down(mid, last);
    }

    c.bench_function("viewer/scroll/10k_lines_mid", |b| {
        b.iter(|| {
            terminal
                .draw(|frame| ui(frame, black_box(&mut app)))
                .expect("draw");
            black_box(terminal.backend().buffer());
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets = bench_viewer_render, bench_viewer_scroll
}
criterion_main!(benches);
