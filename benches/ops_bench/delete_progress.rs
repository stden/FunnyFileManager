//! Purpose: Measure the deletion scan and the real F8 path with progress enabled.

use super::{source_tree, Shape};
use criterion::{BatchSize, Criterion};
use funny_file_manager::models::{App, Overlay, Panel};
use funny_file_manager::ops::handle_delete;
use funny_file_manager::progress::{self, Operation, Painter, Snapshot};
use std::hint::black_box;

/// Keep production progress accounting and throttling without terminal output.
struct NoopPainter;

impl Painter for NoopPainter {
    fn paint(&mut self, _snapshot: &Snapshot) {}
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
pub fn bench_delete_progress(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete_progress");
    group.sample_size(20);
    let _installed = progress::install(Box::new(NoopPainter));

    // Reuse a warm tree so this isolates the progress pre-scan, not creation.
    let root = source_tree(Shape::ManySmall);
    let sources = [root.path().join("tree")];
    group.bench_function("scan_many_small", |b| {
        b.iter(|| {
            let _run = progress::begin(Operation::Delete, black_box(&sources));
        });
    });

    group.bench_function("delete_many_small", |b| {
        b.iter_batched(
            || {
                let root = source_tree(Shape::ManySmall);
                let source = root.path().join("tree");
                let left = Panel::new(root.path().to_path_buf()).expect("left panel");
                let right = Panel::new(root.path().to_path_buf()).expect("right panel");
                let mut app = App::with_panels(left, right);
                app.set_overlay(Overlay::DeleteConfirm(vec![source.clone()]));
                (root, app, source)
            },
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
