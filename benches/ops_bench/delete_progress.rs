//! Purpose: Measure the deletion scan, streaming progress batches, and the F8 path.

use super::{source_tree, Shape};
use criterion::{measurement::WallTime, BatchSize, BenchmarkGroup, Criterion};
use funny_file_manager::models::{App, Overlay, Panel};
use funny_file_manager::ops::handle_delete;
use funny_file_manager::progress::{
    self, begin_delete, delete_progress, Operation, Painter, Snapshot,
};
use std::hint::black_box;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Keep production progress accounting and throttling without terminal output.
pub struct NoopPainter;

impl Painter for NoopPainter {
    fn paint(&mut self, _snapshot: &Snapshot) {}
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
pub fn confirmed_app(root: TempDir) -> (TempDir, App, PathBuf) {
    let source = root.path().join("tree");
    let left = Panel::new(root.path().to_path_buf()).expect("left panel");
    let right = Panel::new(root.path().to_path_buf()).expect("right panel");
    let mut app = App::with_panels(left, right);
    app.set_overlay(Overlay::DeleteConfirm(vec![source.clone()]));
    (root, app, source)
}

fn bench_scan_and_stream(group: &mut BenchmarkGroup<'_, WallTime>, source: &Path) {
    let sources = [source.to_path_buf()];
    group.bench_function("scan_many_small", |b| {
        b.iter(|| {
            let _run = progress::begin(Operation::Delete, black_box(&sources));
        });
    });

    let sample = source.join("file-00000.dat");
    group.bench_function("stream_batches_512", |b| {
        b.iter(|| {
            let _run = begin_delete(black_box(&sources));
            for _ in 0..32_u32 {
                black_box(delete_progress(16, 16, Some(black_box(&sample))));
            }
        });
    });
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
pub fn bench_delete_progress(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete_progress");
    group.sample_size(20);
    let _installed = progress::install(Box::new(NoopPainter));

    // Reuse a warm tree so this isolates progress accounting, not creation.
    let root = source_tree(Shape::ManySmall);
    bench_scan_and_stream(&mut group, &root.path().join("tree"));

    group.bench_function("delete_many_small", |b| {
        b.iter_batched(
            || confirmed_app(source_tree(Shape::ManySmall)),
            |(root, mut app, source)| {
                handle_delete(black_box(&mut app)).expect("delete");
                assert!(!source.exists(), "delete left its source tree");
                // Criterion drops the application and fixture outside the timer.
                black_box((root, app))
            },
            BatchSize::PerIteration,
        );
    });

    group.finish();
}
